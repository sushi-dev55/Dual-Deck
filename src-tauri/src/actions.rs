use crate::domain::{
    ActionDefinition, ActionStep, ExecutionOutcome, InputBinding, WebhookMethod, WebhookRequest,
};
use crate::error::{AppResult, CommandError};
use crate::platform::DesktopPlatform;
use crate::storage::Database;
use reqwest::header::{HeaderName, HeaderValue};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

const MAX_CONCURRENT_ACTIONS: usize = 16;
const MAX_PENDING_ACTIONS: usize = 64;

#[derive(Clone)]
pub struct ActionService {
    worker: Arc<ActionWorker>,
    capacity: ActionCapacity,
}

#[derive(Clone)]
struct ActionCapacity {
    execution: Arc<Semaphore>,
    admission: Arc<Semaphore>,
}

pub(crate) struct ReservedAction {
    worker: Arc<ActionWorker>,
    capacity: ActionCapacity,
    _admission: OwnedSemaphorePermit,
}

struct ActionWorker {
    database: Arc<Database>,
    platform: Arc<dyn DesktopPlatform>,
    web_client: reqwest::Client,
}

impl ActionService {
    pub fn new(database: Arc<Database>, platform: Arc<dyn DesktopPlatform>) -> AppResult<Self> {
        let web_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!("DualDeck/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            worker: Arc::new(ActionWorker {
                database,
                platform,
                web_client,
            }),
            capacity: ActionCapacity::new(MAX_CONCURRENT_ACTIONS, MAX_PENDING_ACTIONS),
        })
    }

    pub async fn execute_binding(&self, binding_id: uuid::Uuid) -> AppResult<ExecutionOutcome> {
        self.try_reserve()?.execute_binding(binding_id).await
    }

    pub(crate) fn try_reserve(&self) -> AppResult<ReservedAction> {
        Ok(ReservedAction {
            worker: Arc::clone(&self.worker),
            capacity: self.capacity.clone(),
            _admission: self.capacity.try_admit()?,
        })
    }
}

impl ActionCapacity {
    fn new(concurrent: usize, pending: usize) -> Self {
        debug_assert!(concurrent > 0);
        debug_assert!(pending >= concurrent);
        Self {
            execution: Arc::new(Semaphore::new(concurrent)),
            admission: Arc::new(Semaphore::new(pending)),
        }
    }

    fn try_admit(&self) -> AppResult<OwnedSemaphorePermit> {
        self.admission
            .clone()
            .try_acquire_owned()
            .map_err(|error| match error {
                TryAcquireError::NoPermits => CommandError::new(
                    "actionQueueFull",
                    "Too many actions are already running or waiting",
                ),
                TryAcquireError::Closed => {
                    CommandError::new("actionServiceStopped", "The action service is not running")
                }
            })
    }

    async fn acquire_if<F>(&self, still_current: F) -> AppResult<Option<OwnedSemaphorePermit>>
    where
        F: Fn() -> bool,
    {
        if !still_current() {
            return Ok(None);
        }
        let permit = self.execution.clone().acquire_owned().await.map_err(|_| {
            CommandError::new("actionServiceStopped", "The action service is not running")
        })?;
        if still_current() {
            Ok(Some(permit))
        } else {
            Ok(None)
        }
    }
}

impl ReservedAction {
    async fn execute_binding(&self, binding_id: uuid::Uuid) -> AppResult<ExecutionOutcome> {
        self.execute_binding_if(binding_id, || true)
            .await?
            .ok_or_else(|| {
                CommandError::new("actionCancelled", "The action was cancelled before it ran")
            })
    }

    pub(crate) async fn execute_binding_if<F>(
        &self,
        binding_id: uuid::Uuid,
        still_current: F,
    ) -> AppResult<Option<ExecutionOutcome>>
    where
        F: Fn() -> bool,
    {
        let Some(_permit) = self.capacity.acquire_if(still_current).await? else {
            return Ok(None);
        };
        let settings = self.worker.database.settings()?;
        if settings.mappings_paused {
            return Err(CommandError::new(
                "mappingsPaused",
                "Mappings are currently paused",
            ));
        }
        let binding = self.worker.database.binding(binding_id)?;
        validate_active_binding(&binding, settings.active_profile_id)?;
        self.worker.execute(&binding.action, 0).await.map(Some)
    }
}

impl ActionWorker {
    fn execute<'a>(
        &'a self,
        action: &'a ActionDefinition,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = AppResult<ExecutionOutcome>> + Send + 'a>> {
        Box::pin(async move {
            if depth > 16 {
                return Err(CommandError::new(
                    "actionNestingTooDeep",
                    "Multi-actions cannot be nested more than 16 levels",
                ));
            }
            match action {
                ActionDefinition::Incomplete { .. } => Err(CommandError::new(
                    "incompleteAction",
                    "Finish configuring this action before running it",
                )),
                ActionDefinition::OpenApplication {
                    path,
                    arguments,
                    working_directory,
                } => {
                    self.platform.launch_application(
                        path,
                        arguments,
                        working_directory.as_deref(),
                    )?;
                    Ok(completed())
                }
                ActionDefinition::OpenPath { path } => {
                    self.platform.open_path(path)?;
                    Ok(completed())
                }
                ActionDefinition::OpenUrl { url } => {
                    let url = validated_web_url(url)?;
                    self.platform.open_url(url.as_str())?;
                    Ok(completed())
                }
                ActionDefinition::Hotkey { hotkey } => {
                    self.platform.send_hotkey(hotkey)?;
                    Ok(completed())
                }
                ActionDefinition::TypeText { text } => {
                    self.platform.type_text(text)?;
                    Ok(completed())
                }
                ActionDefinition::Media { command } => {
                    self.platform.media_command(*command)?;
                    Ok(completed())
                }
                ActionDefinition::Volume { command } => {
                    self.platform.volume_command(*command)?;
                    Ok(completed())
                }
                ActionDefinition::PlaySound { path } => {
                    self.platform.play_sound(path)?;
                    Ok(completed())
                }
                ActionDefinition::Webhook { request } => {
                    self.send_webhook(request).await?;
                    Ok(completed())
                }
                ActionDefinition::CloseApplication { executable_name } => {
                    self.platform.close_application(executable_name)?;
                    Ok(completed())
                }
                ActionDefinition::SwitchProfile { profile_id } => {
                    self.database.set_active_profile(*profile_id)?;
                    Ok(ExecutionOutcome {
                        completed_steps: 1,
                        profile_switched_to: Some(*profile_id),
                    })
                }
                ActionDefinition::Delay { duration_ms } => {
                    delay(*duration_ms).await?;
                    Ok(completed())
                }
                ActionDefinition::MultiAction {
                    steps,
                    stop_on_error,
                } => self.execute_steps(steps, *stop_on_error, depth + 1).await,
            }
        })
    }

    async fn execute_steps(
        &self,
        steps: &[ActionStep],
        stop_on_error: bool,
        depth: usize,
    ) -> AppResult<ExecutionOutcome> {
        if steps.is_empty() || steps.len() > 100 {
            return Err(CommandError::new(
                "invalidMultiAction",
                "Multi-actions must contain between 1 and 100 steps",
            ));
        }
        if steps
            .iter()
            .any(|step| matches!(&step.action, ActionDefinition::SwitchProfile { .. }))
        {
            return Err(CommandError::new(
                "profileSwitchInMultiAction",
                "Profile switching cannot be used inside a multi-action",
            ));
        }
        let mut outcome = ExecutionOutcome {
            completed_steps: 0,
            profile_switched_to: None,
        };
        let mut last_error = None;
        for step in steps {
            match self.execute(&step.action, depth).await {
                Ok(step_outcome) => {
                    outcome.completed_steps += step_outcome.completed_steps;
                    if step_outcome.profile_switched_to.is_some() {
                        outcome.profile_switched_to = step_outcome.profile_switched_to;
                    }
                }
                Err(error) if stop_on_error => return Err(error),
                Err(error) => last_error = Some(error),
            }
            if step.delay_after_ms > 0 {
                delay(step.delay_after_ms).await?;
            }
        }
        if outcome.completed_steps == 0 {
            if let Some(error) = last_error {
                return Err(error);
            }
        }
        Ok(outcome)
    }

    async fn send_webhook(&self, webhook: &WebhookRequest) -> AppResult<()> {
        let url = validated_web_url(&webhook.url)?;
        if webhook.headers.len() > 32 {
            return Err(CommandError::new(
                "tooManyHeaders",
                "Webhooks support at most 32 headers",
            ));
        }
        if webhook
            .body
            .as_ref()
            .is_some_and(|body| body.len() > 1_048_576)
        {
            return Err(CommandError::new(
                "webhookBodyTooLarge",
                "Webhook bodies cannot exceed 1 MB",
            ));
        }
        let method = match webhook.method {
            WebhookMethod::Get => reqwest::Method::GET,
            WebhookMethod::Post => reqwest::Method::POST,
            WebhookMethod::Put => reqwest::Method::PUT,
            WebhookMethod::Patch => reqwest::Method::PATCH,
            WebhookMethod::Delete => reqwest::Method::DELETE,
        };
        let timeout = webhook.timeout_ms.clamp(500, 60_000);
        let mut request = self
            .web_client
            .request(method, url)
            .timeout(Duration::from_millis(timeout));
        for (name, value) in &webhook.headers {
            let name = HeaderName::try_from(name)
                .map_err(|error| CommandError::new("invalidWebhookHeader", error.to_string()))?;
            let value = HeaderValue::try_from(value)
                .map_err(|error| CommandError::new("invalidWebhookHeader", error.to_string()))?;
            request = request.header(name, value);
        }
        if let Some(body) = &webhook.body {
            request = request.body(body.clone());
        }
        let response = request.send().await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(CommandError::new(
                "webhookRejected",
                format!("The webhook returned HTTP {}", response.status().as_u16()),
            ))
        }
    }
}

fn completed() -> ExecutionOutcome {
    ExecutionOutcome {
        completed_steps: 1,
        profile_switched_to: None,
    }
}

async fn delay(duration_ms: u64) -> AppResult<()> {
    if duration_ms > 86_400_000 {
        return Err(CommandError::new(
            "delayTooLong",
            "A delay cannot exceed 24 hours",
        ));
    }
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
    Ok(())
}

fn validated_web_url(value: &str) -> AppResult<url::Url> {
    let url = url::Url::parse(value.trim())?;
    if matches!(url.scheme(), "http" | "https") && url.host().is_some() {
        Ok(url)
    } else {
        Err(CommandError::new(
            "invalidUrl",
            "Only HTTP and HTTPS URLs are supported",
        ))
    }
}

fn validate_active_binding(binding: &InputBinding, active_profile_id: uuid::Uuid) -> AppResult<()> {
    if !binding.enabled {
        return Err(CommandError::new(
            "mappingDisabled",
            "The selected mapping is disabled",
        ));
    }
    if binding.profile_id != active_profile_id {
        return Err(CommandError::new(
            "mappingInactive",
            "The selected mapping is not in the active profile",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn accepts_only_http_urls() {
        assert!(validated_web_url("https://example.com/hook").is_ok());
        assert!(validated_web_url("file:///C:/Windows/System32").is_err());
        assert!(validated_web_url("javascript:alert(1)").is_err());
    }

    #[tokio::test]
    async fn rejects_excessive_delays() {
        assert!(delay(86_400_001).await.is_err());
    }

    #[test]
    fn action_admission_has_a_hard_limit() {
        let capacity = ActionCapacity::new(1, 2);
        let first = capacity.try_admit().expect("first reservation");
        let second = capacity.try_admit().expect("second reservation");
        let error = match capacity.try_admit() {
            Ok(_) => panic!("admission exceeded its configured limit"),
            Err(error) => error,
        };
        assert_eq!(error.code, "actionQueueFull");

        drop(first);
        assert!(capacity.try_admit().is_ok());
        drop(second);
    }

    #[tokio::test]
    async fn cancellation_is_checked_after_execution_capacity_is_available() {
        let capacity = ActionCapacity::new(1, 2);
        let occupied = capacity
            .acquire_if(|| true)
            .await
            .expect("capacity acquisition")
            .expect("active permit");
        let still_current = Arc::new(AtomicBool::new(true));
        let cancellation_checks = Arc::new(AtomicUsize::new(0));
        let waiter_capacity = capacity.clone();
        let waiter_current = Arc::clone(&still_current);
        let waiter_checks = Arc::clone(&cancellation_checks);
        let waiter = tokio::spawn(async move {
            waiter_capacity
                .acquire_if(|| {
                    waiter_checks.fetch_add(1, Ordering::AcqRel);
                    waiter_current.load(Ordering::Acquire)
                })
                .await
                .expect("queued capacity acquisition")
        });

        while cancellation_checks.load(Ordering::Acquire) == 0 {
            tokio::task::yield_now().await;
        }
        assert!(!waiter.is_finished());
        still_current.store(false, Ordering::Release);
        drop(occupied);

        assert!(waiter.await.expect("capacity waiter").is_none());
        assert!(cancellation_checks.load(Ordering::Acquire) >= 2);
    }
}
