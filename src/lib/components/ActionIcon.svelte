<script lang="ts">
  import {
    AppWindow,
    AudioLines,
    CircleDot,
    File,
    Folder,
    GalleryHorizontalEnd,
    Globe,
    Keyboard,
    Layers,
    ListRestart,
    Play,
    Radio,
    SkipBack,
    SkipForward,
    TextCursorInput,
    Timer,
    Volume1,
    Volume2,
    VolumeX,
    Webhook,
  } from '@lucide/svelte';
  import type { ActionIcon as ActionIconName } from '../models';

  interface Props {
    name: ActionIconName;
    size?: number;
    strokeWidth?: number;
  }

  let { name, size = 18, strokeWidth = 1.8 }: Props = $props();
</script>

{#if name === 'app'}
  <AppWindow {size} {strokeWidth} />
{:else if name === 'file'}
  <File {size} {strokeWidth} />
{:else if name === 'folder'}
  <Folder {size} {strokeWidth} />
{:else if name === 'globe'}
  <Globe {size} {strokeWidth} />
{:else if name === 'keyboard'}
  <Keyboard {size} {strokeWidth} />
{:else if name === 'text'}
  <TextCursorInput {size} {strokeWidth} />
{:else if name === 'play'}
  <Play {size} {strokeWidth} />
{:else if name === 'next'}
  <SkipForward {size} {strokeWidth} />
{:else if name === 'previous'}
  <SkipBack {size} {strokeWidth} />
{:else if name === 'volume-up'}
  <Volume2 {size} {strokeWidth} />
{:else if name === 'volume-down'}
  <Volume1 {size} {strokeWidth} />
{:else if name === 'volume-off'}
  <VolumeX {size} {strokeWidth} />
{:else if name === 'layers'}
  <Layers {size} {strokeWidth} />
{:else if name === 'timer'}
  <Timer {size} {strokeWidth} />
{:else if name === 'webhook'}
  <Webhook {size} {strokeWidth} />
{:else if name === 'profile'}
  <ListRestart {size} {strokeWidth} />
{:else if name === 'audio'}
  <AudioLines {size} {strokeWidth} />
{:else if name === 'scene'}
  <GalleryHorizontalEnd {size} {strokeWidth} />
{:else if name === 'broadcast'}
  <Radio {size} {strokeWidth} />
{:else}
  <CircleDot {size} {strokeWidth} />
{/if}
