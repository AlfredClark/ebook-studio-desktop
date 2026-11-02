<script lang="ts">
  // 渲染异常边界：捕获子树的渲染错误，上报 plugin-log（堆栈仅开发环境展示），并提供重试入口。
  // 后端 panic 由 cores/system.rs 的 panic 钩子处理，两者互不替代。
  import { error as logError } from "@tauri-apps/plugin-log";
  import type { Snippet } from "svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
  } from "#components/shadcn-svelte/card";

  let { children }: { children: Snippet } = $props();

  const isDev = import.meta.env.DEV;

  // 确定性渲染崩溃每次重试都会触发上报，此处节流避免刷屏
  // （全局 2s 窗口，异构错误的首次现场以同步控制台留底为准）
  let lastReportAt = 0;

  async function handleBoundaryError(err: unknown) {
    // `svelte:boundary` 的 `onerror` 不等待返回的 Promise，关窗/秒点重试可能丢日志，
    // 此处先同步留底（控制台），再异步走 plugin-log
    console.error(err);
    const now = Date.now();
    if (now - lastReportAt < 2000) return;
    lastReportAt = now;

    const errorDetails =
      err instanceof Error
        ? `[UI Boundary Error] ${err.name}: ${err.message}\nStack:\n${err.stack || "No stack trace"}`
        : `[UI Boundary Error] Non-error object thrown: ${String(err)}`;

    // 调用 tauri_plugin_log 前端绑定 API
    try {
      await logError(errorDetails);
    } catch (pluginErr) {
      // plugin-log 写失败则控制台已留底，此处仅开发环境再提示写失败本身
      if (isDev) console.warn("Failed to write error via tauri_plugin_log:", pluginErr);
    }
  }
</script>

<svelte:boundary onerror={handleBoundaryError}>
  {@render children()}

  {#snippet failed(error, reset)}
    <div class="flex h-screen w-screen items-center justify-center overflow-hidden p-4">
      <Card class="flex h-full max-h-1/2 w-full max-w-3/5 flex-col">
        <CardHeader>
          <CardTitle>组件渲染异常</CardTitle>
          <CardDescription>当前组件无法正常加载。您可以尝试重试或退出。</CardDescription>
        </CardHeader>
        <CardContent class="flex-1 overflow-y-auto">
          {#if isDev}
            <pre class="font-mono text-sm text-wrap text-destructive">{error instanceof Error
                ? `${error.name}: ${error.message}\n${error.stack}`
                : String(error)}</pre>
          {/if}
        </CardContent>
        <CardFooter class="flex w-full flex-row justify-around">
          <Button variant="outline" class="min-w-1/3" onclick={reset}>重试</Button>
        </CardFooter>
      </Card>
    </div>
  {/snippet}
</svelte:boundary>
