<script lang="ts">
  // 通用设置分组：界面语言（后端持久化）与三项开关（自启/窗口记忆/更新检查），零 props，直接读写配置 hook。
  // 语言切换是“先落盘后端、再重载前端”的两段提交，期间禁用下拉并给出 loading 提示。
  import type { CloseBehavior, Locale } from "#libs/commands/types";
  import { configState, updateConfig } from "#libs/hooks/config.svelte";
  import { Switch } from "#components/shadcn-svelte/switch";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "#components/shadcn-svelte/select";
  import { m } from "#libs/i18n/paraglide/messages";
  import { getLocale, setLocale } from "#libs/i18n/paraglide/runtime";
  import { toast } from "#libs/utils/toast";

  /** 语言下拉的候选项：`items` 让触发器在下拉关闭时也能解析出展示文案。 */
  const localeItems: { value: Locale; label: string }[] = [
    { value: "en", label: m.settings_language_option_en() },
    { value: "zh-CN", label: m.settings_language_option_zh() },
  ];

  /** 关闭行为下拉的候选项，同上。 */
  const closeBehaviorItems: { value: CloseBehavior; label: string }[] = [
    { value: "prompt", label: m.settings_close_behavior_option_prompt() },
    { value: "exit", label: m.settings_close_behavior_option_exit() },
    {
      value: "minimize_to_tray",
      label: m.settings_close_behavior_option_minimize_to_tray(),
    },
  ];

  /** 语言切换进行中时禁用下拉，避免重复提交。 */
  let switchingLocale = $state(false);

  /** 自启切换进行中时禁用开关，避免重复提交。 */
  let switchingAutostart = $state(false);

  /** 窗口记忆切换进行中时禁用开关，避免重复提交。 */
  let switchingRememberWindow = $state(false);

  /** 更新检查切换进行中时禁用开关，避免重复提交。 */
  let switchingAutoCheckUpdate = $state(false);

  /** 托盘切换进行中时禁用开关，避免重复提交。 */
  let switchingTray = $state(false);

  // 后端配置是语言的唯一真值；水合完成前用 Paraglide 当前语言兜底，避免首帧闪空。
  const currentLocale = $derived<Locale>(configState.value?.locale ?? getLocale());

  // 开机自启同样以后端配置为真值；水合前按默认值关闭渲染。
  const autoStart = $derived(configState.value?.auto_start ?? false);

  // 记住窗口同样以后端配置为真值；水合前按默认值关闭渲染。
  const rememberWindow = $derived(configState.value?.remember_window ?? false);

  // 自动检查更新同样以后端配置为真值；水合前按默认值关闭渲染。
  const autoCheckUpdate = $derived(configState.value?.auto_check_update ?? false);

  // 系统托盘同样以后端配置为真值；水合前按默认值开启渲染（与后端缺省一致）。
  const trayEnabled = $derived(configState.value?.tray_enabled ?? true);

  // 关闭行为同样以后端配置为真值；水合前按默认值弹窗提示渲染。
  const closeBehavior = $derived<CloseBehavior>(configState.value?.close_behavior ?? "prompt");

  /** 语言先经命令落盘后端，成功后再用 Paraglide 默认重载生效；失败则回滚并提示。 */
  // `bits-ui` 给 `string`，经 `localeItems` 收窄为 `Locale` 后才提交，无类型断言
  async function handleLocaleChange(value: string): Promise<void> {
    const locale = localeItems.find((item) => item.value === value)?.value;
    if (!locale || locale === currentLocale || switchingLocale) return;
    switchingLocale = true;
    // 加载提示不自动消失（后端慢于默认 3s 时仍有反馈），结算后必定关闭
    const toastId = toast.loading(m.settings_language_switching(), { duration: Infinity });
    try {
      if (await updateConfig({ locale })) {
        toast.success(m.settings_language_updated());
        setLocale(locale);
      } else {
        toast.error(m.settings_language_update_failed());
      }
    } finally {
      toast.dismiss(toastId);
      switchingLocale = false;
    }
  }

  /** 开机自启经命令落盘后端并同步操作系统；失败不乐观更新，开关自动回滚。 */
  async function handleAutostartChange(checked: boolean): Promise<void> {
    if (checked === autoStart || switchingAutostart) return;
    switchingAutostart = true;
    try {
      if (await updateConfig({ auto_start: checked })) {
        toast.success(m.settings_autostart_updated());
      } else {
        toast.error(m.settings_autostart_update_failed());
      }
    } finally {
      switchingAutostart = false;
    }
  }

  /** 记住窗口经命令落盘后端，次启动生效；失败不乐观更新，开关自动回滚。 */
  async function handleRememberWindowChange(checked: boolean): Promise<void> {
    if (checked === rememberWindow || switchingRememberWindow) return;
    switchingRememberWindow = true;
    try {
      if (await updateConfig({ remember_window: checked })) {
        toast.success(m.settings_remember_window_updated());
      } else {
        toast.error(m.settings_remember_window_update_failed());
      }
    } finally {
      switchingRememberWindow = false;
    }
  }

  /** 自动检查更新经命令落盘后端，次启动生效；失败不乐观更新，开关自动回滚。 */
  async function handleAutoCheckUpdateChange(checked: boolean): Promise<void> {
    if (checked === autoCheckUpdate || switchingAutoCheckUpdate) return;
    switchingAutoCheckUpdate = true;
    try {
      if (await updateConfig({ auto_check_update: checked })) {
        toast.success(m.settings_auto_check_update_updated());
      } else {
        toast.error(m.settings_auto_check_update_failed());
      }
    } finally {
      switchingAutoCheckUpdate = false;
    }
  }

  /** 系统托盘开关经命令落盘后端，即时显隐图标；失败不乐观更新，开关自动回滚。 */
  // 关闭托盘时若关闭行为为最小化到托盘，后端会自动回落弹窗提示并经写后值回写
  async function handleTrayChange(checked: boolean): Promise<void> {
    if (checked === trayEnabled || switchingTray) return;
    switchingTray = true;
    try {
      if (await updateConfig({ tray_enabled: checked })) {
        toast.success(m.settings_tray_updated());
      } else {
        toast.error(m.settings_tray_update_failed());
      }
    } finally {
      switchingTray = false;
    }
  }

  /** 关闭行为经命令落盘后端，次关闭生效；失败不乐观更新，下拉自动回滚。 */
  // `bits-ui` 给 `string`，经 `closeBehaviorItems` 收窄为 `CloseBehavior` 后才提交，无类型断言
  async function handleCloseBehaviorChange(value: string): Promise<void> {
    const behavior = closeBehaviorItems.find((item) => item.value === value)?.value;
    if (!behavior || behavior === closeBehavior) return;
    if (await updateConfig({ close_behavior: behavior })) {
      toast.success(m.settings_close_behavior_updated());
    } else {
      toast.error(m.settings_close_behavior_update_failed());
    }
  }
</script>

<CardSection title={m.settings_general_title()} description={m.settings_general_description()}>
  <CardRow label={m.settings_language_label()} description={m.settings_language_description()}>
    {#snippet control()}
      <Select
        type="single"
        value={currentLocale}
        items={localeItems}
        onValueChange={handleLocaleChange}
        disabled={switchingLocale}
      >
        <SelectTrigger class="w-(--control-column-width)" aria-label={m.settings_language_label()}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="en">{m.settings_language_option_en()}</SelectItem>
          <SelectItem value="zh-CN">{m.settings_language_option_zh()}</SelectItem>
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
  <CardRow label={m.settings_autostart_label()} description={m.settings_autostart_description()}>
    {#snippet control()}
      <Switch
        checked={autoStart}
        onCheckedChange={handleAutostartChange}
        disabled={switchingAutostart}
        aria-label={m.settings_autostart_label()}
      />
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_remember_window_label()}
    description={m.settings_remember_window_description()}
  >
    {#snippet control()}
      <Switch
        checked={rememberWindow}
        onCheckedChange={handleRememberWindowChange}
        disabled={switchingRememberWindow}
        aria-label={m.settings_remember_window_label()}
      />
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_auto_check_update_label()}
    description={m.settings_auto_check_update_description()}
  >
    {#snippet control()}
      <Switch
        checked={autoCheckUpdate}
        onCheckedChange={handleAutoCheckUpdateChange}
        disabled={switchingAutoCheckUpdate}
        aria-label={m.settings_auto_check_update_label()}
      />
    {/snippet}
  </CardRow>
  <CardRow label={m.settings_tray_label()} description={m.settings_tray_description()}>
    {#snippet control()}
      <Switch
        checked={trayEnabled}
        onCheckedChange={handleTrayChange}
        disabled={switchingTray}
        aria-label={m.settings_tray_label()}
      />
    {/snippet}
  </CardRow>
  <CardRow
    label={m.settings_close_behavior_label()}
    description={m.settings_close_behavior_description()}
  >
    {#snippet control()}
      <Select
        type="single"
        value={closeBehavior}
        items={closeBehaviorItems}
        onValueChange={handleCloseBehaviorChange}
      >
        <SelectTrigger
          class="w-(--control-column-width)"
          aria-label={m.settings_close_behavior_label()}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="prompt">{m.settings_close_behavior_option_prompt()}</SelectItem>
          <SelectItem value="exit">{m.settings_close_behavior_option_exit()}</SelectItem>
          <SelectItem value="minimize_to_tray" disabled={!trayEnabled}>
            {m.settings_close_behavior_option_minimize_to_tray()}
          </SelectItem>
        </SelectContent>
      </Select>
    {/snippet}
  </CardRow>
</CardSection>
