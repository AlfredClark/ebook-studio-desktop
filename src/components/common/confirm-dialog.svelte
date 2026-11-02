<script lang="ts">
  // 通用确认弹窗：标题 + 描述 + 取消/确认双按钮；点确认先关弹窗再调回调。
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from "#components/shadcn-svelte/alert-dialog";
  import type { ButtonVariant } from "#components/shadcn-svelte/button/index.js";

  let {
    open = $bindable(false),
    title,
    description,
    cancelLabel,
    confirmLabel,
    variant = "default",
    onConfirm,
  }: {
    open: boolean;
    title: string;
    description: string;
    cancelLabel: string;
    confirmLabel: string;
    /** 确认按钮的语义变体；破坏性操作（重置 / 删除）必须传 `destructive`，否则会与例行确认长得一样 */
    variant?: ButtonVariant;
    onConfirm: () => unknown;
  } = $props();

  /** 确认先关弹窗再执行业务；回调可为 async，抛错不影响关闭 */
  function handleConfirm(): void {
    open = false;
    void onConfirm();
  }
</script>

<AlertDialog bind:open>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>{title}</AlertDialogTitle>
      <AlertDialogDescription>{description}</AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>{cancelLabel}</AlertDialogCancel>
      <AlertDialogAction {variant} onclick={handleConfirm}>
        {confirmLabel}
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
