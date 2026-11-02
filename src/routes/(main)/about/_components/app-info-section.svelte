<script lang="ts">
  // 应用信息分组：构建常量展示 + 版本检查按钮 + 更新状态面板，零 props。
  // 应用元信息以构建常量为唯一来源（版本号与 tauri.conf.json 一致）。
  import { checkForUpdate, updaterState } from "#libs/hooks/updater.svelte";
  import { Badge } from "#components/shadcn-svelte/badge";
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import UpdaterPanel from "./updater-panel.svelte";

  const appName = __APP_TAURI_CONF__.productName;
  const appVersion = __APP_TAURI_CONF__.version;
  const appDescription = __APP_PKG__.description;
  const appLicense = __APP_PKG__.license;
  const appAuthor = __APP_PKG__.author;
</script>

<CardSection title={m.about_app_info_title()}>
  <CardRow label={m.about_app_name()} value={appName} />

  <CardRow label={m.about_current_version()} value={appVersion}>
    {#snippet control()}
      <div class="flex items-center gap-2">
        {#if updaterState.phase === "checking" || updaterState.phase === "downloading"}
          <Button variant="outline" disabled>
            {updaterState.phase === "checking" ? m.updater_checking() : m.updater_downloading()}
          </Button>
        {:else}
          {#if updaterState.phase === "up-to-date"}
            <Badge variant="secondary">{m.updater_up_to_date()}</Badge>
          {/if}
          <Button variant="outline" onclick={() => void checkForUpdate()}>
            {updaterState.phase === "up-to-date" ? m.updater_recheck() : m.updater_check_update()}
          </Button>
        {/if}
      </div>
    {/snippet}
  </CardRow>

  <UpdaterPanel />

  <CardRow label={m.about_description()} value={appDescription} />

  <CardRow label={m.about_license()} value={appLicense} />

  <CardRow label={m.about_author()} value={appAuthor} />
</CardSection>
