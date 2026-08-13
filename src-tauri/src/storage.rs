use crate::domain::{
    ActionDefinition, AppSettings, AppSnapshot, BindingDraft, ControllerInput, InputBinding,
    Profile, ProfileDraft, Trigger,
};
use crate::error::{AppResult, CommandError};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::fs;
use std::path::Path;
use uuid::Uuid;

const PROFILE_COLUMNS: &str =
    "id, name, description, automatic_app, sort_order, created_at, updated_at";
const BINDING_COLUMNS: &str =
    "id, profile_id, input_json, trigger_json, action_json, label, enabled, created_at, updated_at";
const DATABASE_SCHEMA_VERSION: u32 = 2;
const MAX_PROFILE_NAME_CHARS: usize = 80;
const MAX_PROFILE_DESCRIPTION_CHARS: usize = 4_000;
const MAX_PATH_CHARS: usize = 32_767;
const MAX_ACTION_NODES: usize = 256;
const MAX_URL_CHARS: usize = 8_192;
const MAX_WEBHOOK_HEADER_NAME_BYTES: usize = 256;
const MAX_WEBHOOK_HEADER_VALUE_BYTES: usize = 8_192;
const MAX_WEBHOOK_HEADERS_BYTES: usize = 65_536;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let database = Self {
            connection: Mutex::new(connection),
        };
        database.migrate()?;
        database.ensure_initial_state()?;
        Ok(database)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        let database = Self {
            connection: Mutex::new(connection),
        };
        database.migrate()?;
        database.ensure_initial_state()?;
        Ok(database)
    }

    fn migrate(&self) -> AppResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let version: u32 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > DATABASE_SCHEMA_VERSION {
            return Err(CommandError::new(
                "unsupportedDatabaseVersion",
                format!(
                    "This database was created by a newer version of Dual Deck (schema {version}); this build supports schema {DATABASE_SCHEMA_VERSION}"
                ),
            ));
        }
        if version < 1 {
            transaction.execute_batch(
                "CREATE TABLE profiles (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    description TEXT,
                    automatic_app TEXT,
                    sort_order INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE bindings (
                    id TEXT PRIMARY KEY NOT NULL,
                    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
                    input_json TEXT NOT NULL,
                    trigger_json TEXT NOT NULL,
                    action_json TEXT NOT NULL,
                    label TEXT NOT NULL,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE INDEX bindings_profile_index ON bindings(profile_id);
                CREATE TABLE settings (
                    id INTEGER PRIMARY KEY CHECK(id = 1),
                    data_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                PRAGMA user_version = 1;",
            )?;
        }
        if version < 2 {
            transaction.execute_batch(
                "WITH ranked AS (
                    SELECT id,
                           ROW_NUMBER() OVER (
                               PARTITION BY profile_id, input_json
                               ORDER BY updated_at DESC, id DESC
                           ) AS position
                    FROM bindings
                    WHERE enabled <> 0
                 )
                 UPDATE bindings
                 SET enabled = 0
                 WHERE id IN (SELECT id FROM ranked WHERE position > 1);
                 CREATE UNIQUE INDEX bindings_enabled_input_index
                 ON bindings(profile_id, input_json)
                 WHERE enabled <> 0;
                 PRAGMA user_version = 2;",
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn ensure_initial_state(&self) -> AppResult<()> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let profile_id = transaction
            .query_row(
                "SELECT id FROM profiles ORDER BY sort_order LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let profile_id = match profile_id {
            Some(value) => parse_uuid(&value)?,
            None => {
                let id = Uuid::new_v4();
                let now = Utc::now().to_rfc3339();
                transaction.execute(
                    "INSERT INTO profiles
                     (id, name, description, automatic_app, sort_order, created_at, updated_at)
                     VALUES (?1, 'Default', NULL, NULL, 0, ?2, ?2)",
                    params![id.to_string(), now],
                )?;
                id
            }
        };
        let settings_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE id = 1)",
            [],
            |row| row.get(0),
        )?;
        if !settings_exists {
            let settings = default_settings(profile_id);
            transaction.execute(
                "INSERT INTO settings (id, data_json, updated_at) VALUES (1, ?1, ?2)",
                params![serde_json::to_string(&settings)?, Utc::now().to_rfc3339()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn snapshot(&self) -> AppResult<AppSnapshot> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let profiles = list_profiles_from(&transaction)?;
        let settings = read_settings(&transaction)?;
        let active_profile = profiles
            .iter()
            .find(|profile| profile.id == settings.active_profile_id)
            .cloned()
            .ok_or_else(|| {
                CommandError::new("profileNotFound", "The active profile was not found")
            })?;
        let bindings = list_all_bindings_from(&transaction)?;
        let snapshot = AppSnapshot {
            profiles,
            active_profile,
            bindings,
            settings,
        };
        transaction.commit()?;
        Ok(snapshot)
    }

    #[cfg(test)]
    pub fn list_profiles(&self) -> AppResult<Vec<Profile>> {
        let connection = self.connection.lock();
        list_profiles_from(&connection)
    }

    #[cfg(test)]
    pub fn profile(&self, id: Uuid) -> AppResult<Profile> {
        let connection = self.connection.lock();
        profile_from(&connection, id)
    }

    pub fn create_profile(&self, draft: ProfileDraft) -> AppResult<Profile> {
        let connection = self.connection.lock();
        insert_profile(&connection, draft)
    }

    pub fn update_profile(&self, id: Uuid, draft: ProfileDraft) -> AppResult<Profile> {
        let name = validate_profile_name(&draft.name)?;
        let description = clean_optional(draft.description);
        let automatic_app = draft.automatic_app;
        validate_profile_metadata(description.as_deref(), automatic_app.as_deref())?;
        let now = Utc::now();
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let current = profile_from(&transaction, id)?;
        transaction
            .execute(
                "UPDATE profiles SET name = ?2, description = ?3, automatic_app = ?4,
                 updated_at = ?5 WHERE id = ?1",
                params![
                    id.to_string(),
                    name,
                    description,
                    automatic_app.clone().map(path_to_string),
                    now.to_rfc3339()
                ],
            )
            .map_err(map_constraint)?;
        transaction.commit()?;
        Ok(Profile {
            name,
            description,
            automatic_app,
            updated_at: now,
            ..current
        })
    }

    pub fn duplicate_profile(&self, id: Uuid) -> AppResult<Profile> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let source = profile_from(&transaction, id)?;
        let profiles = list_profiles_from(&transaction)?;
        let bindings = list_bindings_from(&transaction, id)?;
        let name = available_copy_name(&source.name, &profiles);
        let created = insert_profile(
            &transaction,
            ProfileDraft {
                name,
                description: source.description,
                automatic_app: source.automatic_app,
            },
        )?;
        for binding in bindings {
            let mut action = binding.action;
            remap_profile_action(&mut action, id, created.id);
            insert_binding(
                &transaction,
                Uuid::new_v4(),
                BindingDraft {
                    profile_id: created.id,
                    input: binding.input,
                    trigger: binding.trigger,
                    action,
                    label: binding.label,
                    enabled: binding.enabled,
                },
            )?;
        }
        transaction.commit()?;
        Ok(created)
    }

    pub fn delete_profile(&self, id: Uuid) -> AppResult<AppSettings> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let count: i64 =
            transaction.query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))?;
        if count <= 1 {
            return Err(CommandError::new(
                "lastProfile",
                "The only profile cannot be deleted",
            ));
        }
        let referenced_actions = {
            let mut statement = transaction.prepare(
                "SELECT action_json FROM bindings WHERE profile_id <> ?1 ORDER BY created_at",
            )?;
            let rows = statement.query_map([id.to_string()], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for (index, action_json) in referenced_actions.iter().enumerate() {
            let action: ActionDefinition = parse_json_sql(0, action_json)?;
            if action_references_profile(&action, id) {
                return Err(CommandError::new(
                    "profileInUse",
                    format!(
                        "The profile is used by another mapping (reference {}); remove that mapping before deleting the profile",
                        index + 1
                    ),
                ));
            }
        }
        let removed =
            transaction.execute("DELETE FROM profiles WHERE id = ?1", [id.to_string()])?;
        if removed == 0 {
            return Err(CommandError::new(
                "profileNotFound",
                "The profile was not found",
            ));
        }
        let settings_json: String =
            transaction.query_row("SELECT data_json FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })?;
        let mut settings: AppSettings = serde_json::from_str(&settings_json)?;
        if settings.active_profile_id == id {
            let next: String = transaction.query_row(
                "SELECT id FROM profiles ORDER BY sort_order, name COLLATE NOCASE LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            settings.active_profile_id = parse_uuid(&next)?;
            transaction.execute(
                "UPDATE settings SET data_json = ?1, updated_at = ?2 WHERE id = 1",
                params![serde_json::to_string(&settings)?, Utc::now().to_rfc3339()],
            )?;
        }
        transaction.commit()?;
        Ok(settings)
    }

    pub fn list_bindings(&self, profile_id: Uuid) -> AppResult<Vec<InputBinding>> {
        let connection = self.connection.lock();
        list_bindings_from(&connection, profile_id)
    }

    pub fn binding(&self, id: Uuid) -> AppResult<InputBinding> {
        let connection = self.connection.lock();
        binding_from(&connection, id)
    }

    #[cfg(test)]
    pub fn create_binding(&self, draft: BindingDraft) -> AppResult<InputBinding> {
        self.create_binding_with_id(Uuid::new_v4(), draft)
    }

    pub fn upsert_binding(&self, id: Uuid, draft: BindingDraft) -> AppResult<InputBinding> {
        match self.binding(id) {
            Ok(_) => self.update_binding(id, draft),
            Err(error) if error.code == "bindingNotFound" => self.create_binding_with_id(id, draft),
            Err(error) => Err(error),
        }
    }

    fn create_binding_with_id(&self, id: Uuid, draft: BindingDraft) -> AppResult<InputBinding> {
        let connection = self.connection.lock();
        insert_binding(&connection, id, draft)
    }

    pub fn update_binding(&self, id: Uuid, draft: BindingDraft) -> AppResult<InputBinding> {
        validate_binding(&draft)?;
        let label = draft.label.trim().to_string();
        let now = Utc::now();
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let current = binding_from(&transaction, id)?;
        profile_from(&transaction, draft.profile_id)?;
        validate_switch_profile_references_in_database(&transaction, &draft.action)?;
        ensure_input_available(
            &transaction,
            draft.profile_id,
            &draft.input,
            draft.enabled,
            Some(id),
        )?;
        let updated = transaction
            .execute(
                "UPDATE bindings SET profile_id = ?2, input_json = ?3, trigger_json = ?4,
                 action_json = ?5, label = ?6, enabled = ?7, updated_at = ?8 WHERE id = ?1",
                params![
                    id.to_string(),
                    draft.profile_id.to_string(),
                    serde_json::to_string(&draft.input)?,
                    serde_json::to_string(&draft.trigger)?,
                    serde_json::to_string(&draft.action)?,
                    label,
                    draft.enabled,
                    now.to_rfc3339()
                ],
            )
            .map_err(map_binding_constraint)?;
        if updated != 1 {
            return Err(CommandError::new(
                "bindingNotFound",
                "The mapping was not found",
            ));
        }
        transaction.commit()?;
        Ok(InputBinding {
            id,
            profile_id: draft.profile_id,
            input: draft.input,
            trigger: draft.trigger,
            action: draft.action,
            label,
            enabled: draft.enabled,
            created_at: current.created_at,
            updated_at: now,
        })
    }

    pub fn delete_binding(&self, id: Uuid) -> AppResult<()> {
        let connection = self.connection.lock();
        let removed = connection.execute("DELETE FROM bindings WHERE id = ?1", [id.to_string()])?;
        if removed == 0 {
            return Err(CommandError::new(
                "bindingNotFound",
                "The mapping was not found",
            ));
        }
        Ok(())
    }

    pub fn settings(&self) -> AppResult<AppSettings> {
        let connection = self.connection.lock();
        read_settings(&connection)
    }

    pub fn update_settings(&self, requested: AppSettings) -> AppResult<AppSettings> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let mut settings = read_settings(&transaction)?;
        settings.launch_at_startup = requested.launch_at_startup;
        settings.start_minimized = requested.start_minimized;
        settings.minimize_to_tray = requested.minimize_to_tray;
        settings.close_to_tray = requested.close_to_tray;
        settings.check_for_updates = requested.check_for_updates;
        settings.automatic_profile_switching = requested.automatic_profile_switching;
        settings.action_toasts = requested.action_toasts;
        settings.controller_feedback = requested.controller_feedback;
        settings.reduced_motion = requested.reduced_motion;
        settings.update_channel = requested.update_channel;
        write_settings(&transaction, &settings)?;
        transaction.commit()?;
        Ok(settings)
    }

    pub fn set_active_profile(&self, id: Uuid) -> AppResult<AppSettings> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
            [id.to_string()],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(CommandError::new(
                "profileNotFound",
                "The profile was not found",
            ));
        }
        let mut settings = read_settings(&transaction)?;
        settings.active_profile_id = id;
        write_settings(&transaction, &settings)?;
        transaction.commit()?;
        Ok(settings)
    }

    pub fn set_mappings_paused(&self, paused: bool) -> AppResult<AppSettings> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let mut settings = read_settings(&transaction)?;
        settings.mappings_paused = paused;
        write_settings(&transaction, &settings)?;
        transaction.commit()?;
        Ok(settings)
    }

    pub fn toggle_mappings_paused(&self) -> AppResult<AppSettings> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let mut settings = read_settings(&transaction)?;
        settings.mappings_paused = !settings.mappings_paused;
        write_settings(&transaction, &settings)?;
        transaction.commit()?;
        Ok(settings)
    }
}

fn list_profiles_from(connection: &Connection) -> AppResult<Vec<Profile>> {
    let query =
        format!("SELECT {PROFILE_COLUMNS} FROM profiles ORDER BY sort_order, name COLLATE NOCASE");
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([], profile_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn profile_from(connection: &Connection, id: Uuid) -> AppResult<Profile> {
    let query = format!("SELECT {PROFILE_COLUMNS} FROM profiles WHERE id = ?1");
    connection
        .query_row(&query, [id.to_string()], profile_from_row)
        .optional()?
        .ok_or_else(|| CommandError::new("profileNotFound", "The profile was not found"))
}

fn list_bindings_from(connection: &Connection, profile_id: Uuid) -> AppResult<Vec<InputBinding>> {
    let query =
        format!("SELECT {BINDING_COLUMNS} FROM bindings WHERE profile_id = ?1 ORDER BY created_at");
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([profile_id.to_string()], binding_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn list_all_bindings_from(connection: &Connection) -> AppResult<Vec<InputBinding>> {
    let query =
        format!("SELECT {BINDING_COLUMNS} FROM bindings ORDER BY profile_id, created_at, id");
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([], binding_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn binding_from(connection: &Connection, id: Uuid) -> AppResult<InputBinding> {
    let query = format!("SELECT {BINDING_COLUMNS} FROM bindings WHERE id = ?1");
    connection
        .query_row(&query, [id.to_string()], binding_from_row)
        .optional()?
        .ok_or_else(|| CommandError::new("bindingNotFound", "The mapping was not found"))
}

fn insert_profile(connection: &Connection, draft: ProfileDraft) -> AppResult<Profile> {
    let name = validate_profile_name(&draft.name)?;
    let description = clean_optional(draft.description);
    let automatic_app = draft.automatic_app;
    validate_profile_metadata(description.as_deref(), automatic_app.as_deref())?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let sort_order: i64 = connection.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM profiles",
        [],
        |row| row.get(0),
    )?;
    connection
        .execute(
            "INSERT INTO profiles
             (id, name, description, automatic_app, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                id.to_string(),
                name,
                description,
                automatic_app.clone().map(path_to_string),
                sort_order,
                now.to_rfc3339()
            ],
        )
        .map_err(map_constraint)?;
    Ok(Profile {
        id,
        name,
        description,
        automatic_app,
        sort_order,
        created_at: now,
        updated_at: now,
    })
}

fn insert_binding(
    connection: &Connection,
    id: Uuid,
    draft: BindingDraft,
) -> AppResult<InputBinding> {
    profile_from(connection, draft.profile_id)?;
    validate_binding(&draft)?;
    validate_switch_profile_references_in_database(connection, &draft.action)?;
    ensure_input_available(
        connection,
        draft.profile_id,
        &draft.input,
        draft.enabled,
        None,
    )?;
    let now = Utc::now();
    let label = draft.label.trim().to_string();
    connection
        .execute(
            "INSERT INTO bindings
             (id, profile_id, input_json, trigger_json, action_json, label, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                id.to_string(),
                draft.profile_id.to_string(),
                serde_json::to_string(&draft.input)?,
                serde_json::to_string(&draft.trigger)?,
                serde_json::to_string(&draft.action)?,
                label,
                draft.enabled,
                now.to_rfc3339()
            ],
        )
        .map_err(map_binding_constraint)?;
    Ok(InputBinding {
        id,
        profile_id: draft.profile_id,
        input: draft.input,
        trigger: draft.trigger,
        action: draft.action,
        label,
        enabled: draft.enabled,
        created_at: now,
        updated_at: now,
    })
}

fn ensure_input_available(
    connection: &Connection,
    profile_id: Uuid,
    input: &ControllerInput,
    enabled: bool,
    excluding_id: Option<Uuid>,
) -> AppResult<()> {
    if !enabled {
        return Ok(());
    }
    let input_json = serde_json::to_string(input)?;
    let conflict_exists: bool = connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM bindings
             WHERE profile_id = ?1 AND input_json = ?2 AND enabled <> 0
               AND (?3 IS NULL OR id <> ?3)
         )",
        params![
            profile_id.to_string(),
            input_json,
            excluding_id.map(|id| id.to_string())
        ],
        |row| row.get(0),
    )?;
    if conflict_exists {
        Err(CommandError::new(
            "inputAlreadyMapped",
            "This controller input already has an enabled mapping in the profile",
        ))
    } else {
        Ok(())
    }
}

fn default_settings(profile_id: Uuid) -> AppSettings {
    AppSettings {
        active_profile_id: profile_id,
        launch_at_startup: !cfg!(debug_assertions),
        start_minimized: true,
        close_to_tray: true,
        mappings_paused: false,
        check_for_updates: false,
        automatic_profile_switching: false,
        minimize_to_tray: true,
        action_toasts: true,
        controller_feedback: false,
        reduced_motion: false,
        update_channel: Default::default(),
    }
}

fn read_settings(connection: &Connection) -> AppResult<AppSettings> {
    let data: String =
        connection.query_row("SELECT data_json FROM settings WHERE id = 1", [], |row| {
            row.get(0)
        })?;
    Ok(serde_json::from_str(&data)?)
}

fn write_settings(connection: &Connection, settings: &AppSettings) -> AppResult<()> {
    connection.execute(
        "UPDATE settings SET data_json = ?1, updated_at = ?2 WHERE id = 1",
        params![serde_json::to_string(settings)?, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn profile_from_row(row: &Row<'_>) -> rusqlite::Result<Profile> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(5)?;
    let updated_at: String = row.get(6)?;
    Ok(Profile {
        id: parse_uuid_sql(0, &id)?,
        name: row.get(1)?,
        description: row.get(2)?,
        automatic_app: row.get::<_, Option<String>>(3)?.map(Into::into),
        sort_order: row.get(4)?,
        created_at: parse_datetime_sql(5, &created_at)?,
        updated_at: parse_datetime_sql(6, &updated_at)?,
    })
}

fn binding_from_row(row: &Row<'_>) -> rusqlite::Result<InputBinding> {
    let id: String = row.get(0)?;
    let profile_id: String = row.get(1)?;
    let input: String = row.get(2)?;
    let trigger: String = row.get(3)?;
    let action: String = row.get(4)?;
    let created_at: String = row.get(7)?;
    let updated_at: String = row.get(8)?;
    Ok(InputBinding {
        id: parse_uuid_sql(0, &id)?,
        profile_id: parse_uuid_sql(1, &profile_id)?,
        input: parse_json_sql(2, &input)?,
        trigger: parse_json_sql(3, &trigger)?,
        action: parse_json_sql(4, &action)?,
        label: row.get(5)?,
        enabled: row.get(6)?,
        created_at: parse_datetime_sql(7, &created_at)?,
        updated_at: parse_datetime_sql(8, &updated_at)?,
    })
}

fn parse_uuid(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value)
        .map_err(|error| CommandError::new("invalidStoredUuid", error.to_string()))
}

fn parse_uuid_sql(index: usize, value: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn parse_datetime_sql(index: usize, value: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn parse_json_sql<T: serde::de::DeserializeOwned>(
    index: usize,
    value: &str,
) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn validate_profile_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_PROFILE_NAME_CHARS {
        return Err(CommandError::new(
            "invalidProfileName",
            format!("Profile names must contain between 1 and {MAX_PROFILE_NAME_CHARS} characters"),
        ));
    }
    Ok(name.to_string())
}

fn validate_binding(draft: &BindingDraft) -> AppResult<()> {
    validate_binding_with_limit(draft, MAX_ACTION_NODES).map(|_| ())
}

fn validate_binding_with_limit(draft: &BindingDraft, max_action_nodes: usize) -> AppResult<usize> {
    if draft.label.trim().is_empty() || draft.label.chars().count() > 120 {
        return Err(CommandError::new(
            "invalidBindingLabel",
            "Mapping labels must contain between 1 and 120 characters",
        ));
    }
    match &draft.input {
        ControllerInput::Button(_) => {}
        ControllerInput::Combination(buttons) => {
            let unique: std::collections::HashSet<_> = buttons.iter().collect();
            if !(2..=5).contains(&buttons.len()) || unique.len() != buttons.len() {
                return Err(CommandError::new(
                    "invalidButtonCombination",
                    "Button combinations must contain between 2 and 5 distinct buttons",
                ));
            }
        }
        ControllerInput::StickDirection(_)
        | ControllerInput::TriggerZone(_)
        | ControllerInput::TouchpadZone(_) => {
            return Err(CommandError::new(
                "unsupportedControllerInput",
                "This controller input is not supported in the current release",
            ));
        }
    }
    match &draft.trigger {
        Trigger::LongPress { duration_ms } if !(100..=60_000).contains(duration_ms) => {
            return Err(CommandError::new(
                "invalidTriggerTiming",
                "Long-press duration must be between 100 ms and 60 seconds",
            ));
        }
        Trigger::DoublePress { interval_ms } if !(100..=2_000).contains(interval_ms) => {
            return Err(CommandError::new(
                "invalidTriggerTiming",
                "Double-press interval must be between 100 ms and 2 seconds",
            ));
        }
        Trigger::HoldRepeat {
            initial_delay_ms,
            interval_ms,
        } if *initial_delay_ms > 60_000 || !(25..=60_000).contains(interval_ms) => {
            return Err(CommandError::new(
                "invalidTriggerTiming",
                "Hold-repeat timings are outside the supported range",
            ));
        }
        _ => {}
    }
    let mut action_nodes = 0usize;
    validate_action(&draft.action, 0, &mut action_nodes, max_action_nodes)?;
    Ok(action_nodes)
}

fn validate_action(
    action: &ActionDefinition,
    depth: usize,
    action_nodes: &mut usize,
    max_action_nodes: usize,
) -> AppResult<()> {
    if depth > 16 {
        return Err(CommandError::new(
            "actionNestingTooDeep",
            "Multi-actions cannot be nested more than 16 levels",
        ));
    }
    *action_nodes = action_nodes
        .checked_add(1)
        .ok_or_else(|| CommandError::new("actionTooComplex", "The action tree is too large"))?;
    if *action_nodes > max_action_nodes {
        return Err(CommandError::new(
            "actionTooComplex",
            format!("An action cannot contain more than {max_action_nodes} nodes"),
        ));
    }
    match action {
        ActionDefinition::Incomplete {
            action_id,
            configuration,
        } if action_id.trim().is_empty()
            || action_id.chars().count() > 80
            || configuration.to_string().len() > 1_048_576 =>
        {
            Err(CommandError::new(
                "invalidIncompleteAction",
                "The incomplete action draft is invalid",
            ))
        }
        ActionDefinition::OpenApplication {
            path,
            arguments,
            working_directory,
        } => {
            validate_path(path, "Application path")?;
            if let Some(working_directory) = working_directory {
                validate_path(working_directory, "Working directory")?;
            }
            if arguments.len() > 64 || arguments.iter().any(|argument| argument.len() > 4096) {
                return Err(CommandError::new(
                    "invalidArguments",
                    "The application argument list is too large",
                ));
            }
            Ok(())
        }
        ActionDefinition::OpenPath { path } => validate_path(path, "Path"),
        ActionDefinition::OpenUrl { url } => validate_http_url(url),
        ActionDefinition::Hotkey { hotkey }
            if hotkey.key.trim().is_empty()
                || hotkey.key.chars().count() > 64
                || hotkey.modifiers.len() > 4
                || hotkey
                    .modifiers
                    .iter()
                    .enumerate()
                    .any(|(index, modifier)| hotkey.modifiers[index + 1..].contains(modifier)) =>
        {
            Err(CommandError::new(
                "invalidHotkey",
                "A hotkey must contain a key and distinct supported modifiers",
            ))
        }
        ActionDefinition::TypeText { text } if text.chars().count() > 10_000 => Err(
            CommandError::new("textTooLong", "Typed text cannot exceed 10,000 characters"),
        ),
        ActionDefinition::Webhook { request } => {
            validate_http_url(&request.url)?;
            if request.headers.len() > 32 {
                return Err(CommandError::new(
                    "invalidWebhook",
                    "A webhook cannot contain more than 32 headers",
                ));
            }
            let mut header_bytes = 0usize;
            for (name, value) in &request.headers {
                header_bytes = header_bytes
                    .checked_add(name.len())
                    .and_then(|total| total.checked_add(value.len()))
                    .ok_or_else(|| {
                        CommandError::new("invalidWebhook", "The webhook headers are too large")
                    })?;
                if name.len() > MAX_WEBHOOK_HEADER_NAME_BYTES
                    || value.len() > MAX_WEBHOOK_HEADER_VALUE_BYTES
                    || reqwest::header::HeaderName::from_bytes(name.as_bytes()).is_err()
                    || value.parse::<reqwest::header::HeaderValue>().is_err()
                {
                    return Err(CommandError::new(
                        "invalidWebhook",
                        "A webhook contains an invalid or oversized header",
                    ));
                }
            }
            if header_bytes > MAX_WEBHOOK_HEADERS_BYTES
                || request
                    .body
                    .as_ref()
                    .is_some_and(|body| body.len() > 1_048_576)
            {
                return Err(CommandError::new(
                    "invalidWebhook",
                    "The webhook exceeds the supported header or body limit",
                ));
            }
            Ok(())
        }
        ActionDefinition::PlaySound { path } => validate_path(path, "Sound path"),
        ActionDefinition::CloseApplication { executable_name }
            if executable_name.trim().is_empty() || executable_name.chars().count() > 260 =>
        {
            Err(CommandError::new(
                "invalidExecutableName",
                "Executable names must contain between 1 and 260 characters",
            ))
        }
        ActionDefinition::SwitchProfile { .. } if depth > 0 => Err(CommandError::new(
            "nestedProfileSwitch",
            "Profile switching cannot be nested inside a multi-action",
        )),
        ActionDefinition::Delay { duration_ms } if *duration_ms > 86_400_000 => Err(
            CommandError::new("delayTooLong", "A delay cannot exceed 24 hours"),
        ),
        ActionDefinition::MultiAction { steps, .. } => {
            if steps.is_empty() || steps.len() > 100 {
                return Err(CommandError::new(
                    "invalidMultiAction",
                    "Multi-actions must contain between 1 and 100 steps",
                ));
            }
            for step in steps {
                if step.delay_after_ms > 86_400_000 {
                    return Err(CommandError::new(
                        "delayTooLong",
                        "A delay cannot exceed 24 hours",
                    ));
                }
                validate_action(&step.action, depth + 1, action_nodes, max_action_nodes)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_http_url(value: &str) -> AppResult<()> {
    if value.trim().chars().count() > MAX_URL_CHARS {
        return Err(CommandError::new(
            "invalidUrl",
            format!("URLs cannot contain more than {MAX_URL_CHARS} characters"),
        ));
    }
    let url = url::Url::parse(value.trim())?;
    if matches!(url.scheme(), "http" | "https") && url.host().is_some() {
        Ok(())
    } else {
        Err(CommandError::new(
            "invalidUrl",
            "Only HTTP and HTTPS URLs are supported",
        ))
    }
}

fn validate_profile_metadata(
    description: Option<&str>,
    automatic_app: Option<&Path>,
) -> AppResult<()> {
    if description.is_some_and(|value| value.chars().count() > MAX_PROFILE_DESCRIPTION_CHARS) {
        return Err(CommandError::new(
            "profileDescriptionTooLong",
            format!(
                "Profile descriptions cannot contain more than {MAX_PROFILE_DESCRIPTION_CHARS} characters"
            ),
        ));
    }
    if let Some(path) = automatic_app {
        validate_path(path, "Automatic profile application")?;
    }
    Ok(())
}

fn validate_path(path: &Path, label: &str) -> AppResult<()> {
    let value = path.as_os_str().to_string_lossy();
    if value.trim().is_empty() || value.chars().count() > MAX_PATH_CHARS {
        Err(CommandError::new(
            "invalidPath",
            format!("{label} must contain between 1 and {MAX_PATH_CHARS} characters"),
        ))
    } else {
        Ok(())
    }
}

fn remap_profile_action(action: &mut ActionDefinition, source: Uuid, destination: Uuid) {
    match action {
        ActionDefinition::SwitchProfile { profile_id } if *profile_id == source => {
            *profile_id = destination;
        }
        ActionDefinition::MultiAction { steps, .. } => {
            for step in steps {
                remap_profile_action(&mut step.action, source, destination);
            }
        }
        _ => {}
    }
}

fn action_references_profile(action: &ActionDefinition, target: Uuid) -> bool {
    match action {
        ActionDefinition::SwitchProfile { profile_id } => *profile_id == target,
        ActionDefinition::MultiAction { steps, .. } => steps
            .iter()
            .any(|step| action_references_profile(&step.action, target)),
        _ => false,
    }
}

fn validate_switch_profile_references_in_database(
    connection: &Connection,
    action: &ActionDefinition,
) -> AppResult<()> {
    match action {
        ActionDefinition::SwitchProfile { profile_id } => {
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
                [profile_id.to_string()],
                |row| row.get(0),
            )?;
            if exists {
                Ok(())
            } else {
                Err(CommandError::new(
                    "invalidProfileReference",
                    "The mapping refers to a profile that does not exist",
                ))
            }
        }
        ActionDefinition::MultiAction { steps, .. } => {
            for step in steps {
                validate_switch_profile_references_in_database(connection, &step.action)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn path_to_string(path: std::path::PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn available_copy_name(source: &str, profiles: &[Profile]) -> String {
    available_derived_name(source, " Copy", profiles)
}

fn available_derived_name(source: &str, marker: &str, profiles: &[Profile]) -> String {
    for number in 1..=profiles.len() + 2 {
        let suffix = if number == 1 {
            marker.to_string()
        } else {
            format!("{marker} {number}")
        };
        let candidate = name_with_suffix(source, &suffix);
        if !profiles
            .iter()
            .any(|profile| profile.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    name_with_suffix(source, &format!(" {0}", Uuid::new_v4()))
}

fn name_with_suffix(source: &str, suffix: &str) -> String {
    let suffix_chars = suffix.chars().count();
    let source_limit = MAX_PROFILE_NAME_CHARS.saturating_sub(suffix_chars);
    let mut prefix = source.chars().take(source_limit).collect::<String>();
    while prefix.chars().last().is_some_and(char::is_whitespace) {
        prefix.pop();
    }
    if prefix.is_empty() {
        suffix.trim().chars().take(MAX_PROFILE_NAME_CHARS).collect()
    } else {
        format!("{prefix}{suffix}")
    }
}

fn map_constraint(error: rusqlite::Error) -> CommandError {
    match &error {
        rusqlite::Error::SqliteFailure(details, _)
            if details.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            CommandError::new(
                "profileNameInUse",
                "A profile with this name already exists",
            )
        }
        _ => error.into(),
    }
}

fn map_binding_constraint(error: rusqlite::Error) -> CommandError {
    match &error {
        rusqlite::Error::SqliteFailure(details, Some(message))
            if details.code == rusqlite::ErrorCode::ConstraintViolation
                && message.contains("bindings.profile_id, bindings.input_json") =>
        {
            CommandError::new(
                "inputAlreadyMapped",
                "This controller input already has an enabled mapping in the profile",
            )
        }
        _ => error.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ActionDefinition, ActionStep, ControllerButton, ControllerInput, StickDirection, Trigger,
        WebhookMethod, WebhookRequest,
    };
    use std::collections::BTreeMap;

    fn database() -> Database {
        Database::open_in_memory().expect("in-memory database")
    }

    fn create_profile(database: &Database, name: &str) -> Profile {
        database
            .create_profile(ProfileDraft {
                name: name.into(),
                description: None,
                automatic_app: None,
            })
            .expect("profile")
    }

    fn url_binding(profile_id: Uuid, input: ControllerButton, label: &str) -> BindingDraft {
        BindingDraft {
            profile_id,
            input: ControllerInput::Button(input),
            trigger: Trigger::Press,
            action: ActionDefinition::OpenUrl {
                url: "https://example.com".into(),
            },
            label: label.into(),
            enabled: true,
        }
    }

    fn install_binding_failure_trigger(database: &Database, label: &str) {
        let connection = database.connection.lock();
        connection
            .execute_batch(&format!(
                "CREATE TRIGGER fail_selected_binding
                 BEFORE INSERT ON bindings
                 WHEN NEW.label = '{}'
                 BEGIN
                     SELECT RAISE(ABORT, 'forced binding failure');
                 END;",
                label.replace('\'', "''")
            ))
            .expect("failure trigger");
    }

    #[test]
    fn creates_default_profile_and_settings() {
        let snapshot = database().snapshot().expect("snapshot");
        assert_eq!(snapshot.profiles.len(), 1);
        assert_eq!(snapshot.active_profile.name, "Default");
        assert_eq!(snapshot.settings.launch_at_startup, !cfg!(debug_assertions));
        assert!(snapshot.settings.start_minimized);
        assert!(!snapshot.settings.check_for_updates);
    }

    #[test]
    fn profile_crud_preserves_bindings() {
        let database = database();
        let profile = database
            .create_profile(ProfileDraft {
                name: "Streaming".into(),
                description: None,
                automatic_app: None,
            })
            .expect("profile");
        let binding = database
            .create_binding(BindingDraft {
                profile_id: profile.id,
                input: ControllerInput::Button(ControllerButton::Triangle),
                trigger: Trigger::Press,
                action: ActionDefinition::OpenUrl {
                    url: "https://example.com".into(),
                },
                label: "Website".into(),
                enabled: true,
            })
            .expect("binding");
        assert_eq!(
            database.binding(binding.id).expect("stored").label,
            "Website"
        );
        let duplicate = database.duplicate_profile(profile.id).expect("duplicate");
        assert_eq!(
            database
                .list_bindings(duplicate.id)
                .expect("bindings")
                .len(),
            1
        );
        database.delete_profile(profile.id).expect("delete");
        assert!(database.binding(binding.id).is_err());
    }

    #[test]
    fn refuses_to_delete_last_profile() {
        let database = database();
        let id = database.settings().expect("settings").active_profile_id;
        let error = database.delete_profile(id).expect_err("must fail");
        assert_eq!(error.code, "lastProfile");
    }

    #[test]
    fn preference_updates_preserve_active_profile_and_pause_state() {
        let database = database();
        let mut stale = database.settings().expect("settings");
        let profile = database
            .create_profile(ProfileDraft {
                name: "Work".into(),
                description: None,
                automatic_app: None,
            })
            .expect("profile");
        database
            .set_active_profile(profile.id)
            .expect("active profile");
        database.set_mappings_paused(true).expect("pause");

        stale.close_to_tray = false;
        let updated = database.update_settings(stale).expect("preferences");

        assert_eq!(updated.active_profile_id, profile.id);
        assert!(updated.mappings_paused);
        assert!(!updated.close_to_tray);
    }

    #[test]
    fn rejects_reserved_controller_inputs() {
        let database = database();
        let profile_id = database.settings().expect("settings").active_profile_id;
        let error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::StickDirection(StickDirection::LeftUp),
                trigger: Trigger::Press,
                action: ActionDefinition::OpenUrl {
                    url: "https://example.com".into(),
                },
                label: "Unsupported".into(),
                enabled: true,
            })
            .expect_err("reserved input");
        assert_eq!(error.code, "unsupportedControllerInput");
    }

    #[test]
    fn rejects_database_versions_newer_than_supported() {
        let connection = Connection::open_in_memory().expect("connection");
        connection
            .pragma_update(None, "user_version", DATABASE_SCHEMA_VERSION + 1)
            .expect("future schema version");
        let database = Database {
            connection: Mutex::new(connection),
        };

        let error = database.migrate().expect_err("future database must fail");

        assert_eq!(error.code, "unsupportedDatabaseVersion");
    }

    #[test]
    fn migration_disables_older_duplicate_inputs_and_enforces_uniqueness() {
        let database = database();
        let profile_id = database.settings().expect("settings").active_profile_id;
        let older = database
            .create_binding(url_binding(profile_id, ControllerButton::Triangle, "Older"))
            .expect("older binding");
        let newer_id = Uuid::new_v4();
        {
            let connection = database.connection.lock();
            connection
                .execute_batch(
                    "DROP INDEX bindings_enabled_input_index;
                     PRAGMA user_version = 1;",
                )
                .expect("restore version one shape");
            connection
                .execute(
                    "INSERT INTO bindings
                     (id, profile_id, input_json, trigger_json, action_json, label, enabled, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'Newer', 1, '2030-01-01T00:00:00Z', '2030-01-01T00:00:00Z')",
                    params![
                        newer_id.to_string(),
                        profile_id.to_string(),
                        serde_json::to_string(&ControllerInput::Button(
                            ControllerButton::Triangle
                        ))
                        .expect("input"),
                        serde_json::to_string(&Trigger::Press).expect("trigger"),
                        serde_json::to_string(&ActionDefinition::OpenUrl {
                            url: "https://example.com/newer".into()
                        })
                        .expect("action")
                    ],
                )
                .expect("duplicate legacy binding");
        }

        database.migrate().expect("version two migration");

        assert!(!database.binding(older.id).expect("older stored").enabled);
        assert!(database.binding(newer_id).expect("newer stored").enabled);
        let error = database
            .create_binding(url_binding(profile_id, ControllerButton::Triangle, "Third"))
            .expect_err("duplicate active input");
        assert_eq!(error.code, "inputAlreadyMapped");
        let disabled = database
            .create_binding(BindingDraft {
                enabled: false,
                ..url_binding(profile_id, ControllerButton::Triangle, "Disabled draft")
            })
            .expect("disabled duplicate");
        assert!(!disabled.enabled);
    }

    #[test]
    fn duplicate_profile_rolls_back_when_a_binding_cannot_be_copied() {
        let database = database();
        let source = create_profile(&database, "Source");
        database
            .create_binding(url_binding(source.id, ControllerButton::Triangle, "First"))
            .expect("first binding");
        database
            .create_binding(url_binding(
                source.id,
                ControllerButton::Circle,
                "Fail during copy",
            ))
            .expect("second binding");
        install_binding_failure_trigger(&database, "Fail during copy");
        let profile_count = database.list_profiles().expect("profiles").len();

        database
            .duplicate_profile(source.id)
            .expect_err("copy must fail");

        let profiles = database.list_profiles().expect("profiles after failure");
        assert_eq!(profiles.len(), profile_count);
        assert!(!profiles.iter().any(|profile| profile.name.contains("Copy")));
    }

    #[test]
    fn snapshot_returns_all_bindings_in_one_coherent_view() {
        let database = database();
        let active_id = database.settings().expect("settings").active_profile_id;
        let other = create_profile(&database, "Other");
        let binding = database
            .create_binding(url_binding(
                other.id,
                ControllerButton::Triangle,
                "Other action",
            ))
            .expect("other binding");

        let snapshot = database.snapshot().expect("snapshot");

        assert_eq!(snapshot.active_profile.id, active_id);
        assert_eq!(snapshot.bindings, vec![binding]);
    }

    #[test]
    fn refuses_to_delete_a_profile_referenced_by_another_mapping() {
        let database = database();
        let target = create_profile(&database, "Target");
        let source = create_profile(&database, "Source");
        let binding = database
            .create_binding(BindingDraft {
                profile_id: source.id,
                input: ControllerInput::Button(ControllerButton::Triangle),
                trigger: Trigger::Press,
                action: ActionDefinition::SwitchProfile {
                    profile_id: target.id,
                },
                label: "Switch to target".into(),
                enabled: true,
            })
            .expect("profile switch");

        let error = database
            .delete_profile(target.id)
            .expect_err("referenced profile must remain");

        assert_eq!(error.code, "profileInUse");
        assert_eq!(
            database.profile(target.id).expect("target remains").id,
            target.id
        );
        database
            .delete_binding(binding.id)
            .expect("remove reference");
        database.delete_profile(target.id).expect("delete target");
    }

    #[test]
    fn generated_profile_names_never_exceed_the_name_limit() {
        let database = database();
        let source_name = "A".repeat(MAX_PROFILE_NAME_CHARS);
        let source = create_profile(&database, &source_name);

        let duplicate = database.duplicate_profile(source.id).expect("duplicate");

        assert!(duplicate.name.chars().count() <= MAX_PROFILE_NAME_CHARS);
    }

    #[test]
    fn rejects_oversized_webhook_headers() {
        let database = database();
        let profile_id = database.settings().expect("settings").active_profile_id;
        let mut headers = BTreeMap::new();
        headers.insert(
            "x-large".into(),
            "x".repeat(MAX_WEBHOOK_HEADER_VALUE_BYTES + 1),
        );
        let header_error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::Button(ControllerButton::Circle),
                trigger: Trigger::Press,
                action: ActionDefinition::Webhook {
                    request: WebhookRequest {
                        url: "https://example.com/hook".into(),
                        method: WebhookMethod::Post,
                        headers,
                        body: None,
                        timeout_ms: 10_000,
                    },
                },
                label: "Webhook".into(),
                enabled: true,
            })
            .expect_err("oversized webhook header");
        assert_eq!(header_error.code, "invalidWebhook");
    }

    #[test]
    fn rejects_oversized_paths_and_urls() {
        let database = database();
        let profile_id = database.settings().expect("settings").active_profile_id;
        let path_error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::Button(ControllerButton::Triangle),
                trigger: Trigger::Press,
                action: ActionDefinition::OpenApplication {
                    path: std::path::PathBuf::from("x".repeat(MAX_PATH_CHARS + 1)),
                    arguments: Vec::new(),
                    working_directory: None,
                },
                label: "Oversized path".into(),
                enabled: true,
            })
            .expect_err("oversized path");
        assert_eq!(path_error.code, "invalidPath");

        let url_error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::Button(ControllerButton::Circle),
                trigger: Trigger::Press,
                action: ActionDefinition::OpenUrl {
                    url: format!("https://example.com/{}", "x".repeat(MAX_URL_CHARS)),
                },
                label: "Oversized URL".into(),
                enabled: true,
            })
            .expect_err("oversized URL");
        assert_eq!(url_error.code, "invalidUrl");
    }

    #[test]
    fn rejects_oversized_action_trees_and_nested_profile_switches() {
        let database = database();
        let profile_id = database.settings().expect("settings").active_profile_id;
        let nested_group = ActionDefinition::MultiAction {
            steps: (0..3)
                .map(|_| ActionStep {
                    action: ActionDefinition::Delay { duration_ms: 0 },
                    delay_after_ms: 0,
                })
                .collect(),
            stop_on_error: true,
        };
        let oversized = ActionDefinition::MultiAction {
            steps: (0..100)
                .map(|_| ActionStep {
                    action: nested_group.clone(),
                    delay_after_ms: 0,
                })
                .collect(),
            stop_on_error: true,
        };
        let tree_error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::Button(ControllerButton::Triangle),
                trigger: Trigger::Press,
                action: oversized,
                label: "Oversized tree".into(),
                enabled: true,
            })
            .expect_err("oversized action tree");
        assert_eq!(tree_error.code, "actionTooComplex");

        let destination = create_profile(&database, "Destination");
        let nested_switch_error = database
            .create_binding(BindingDraft {
                profile_id,
                input: ControllerInput::Button(ControllerButton::Circle),
                trigger: Trigger::Press,
                action: ActionDefinition::MultiAction {
                    steps: vec![ActionStep {
                        action: ActionDefinition::SwitchProfile {
                            profile_id: destination.id,
                        },
                        delay_after_ms: 0,
                    }],
                    stop_on_error: true,
                },
                label: "Nested switch".into(),
                enabled: true,
            })
            .expect_err("nested profile switch");
        assert_eq!(nested_switch_error.code, "nestedProfileSwitch");
    }
}
