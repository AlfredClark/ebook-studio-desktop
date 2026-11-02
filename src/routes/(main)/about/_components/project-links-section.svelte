<script lang="ts">
  // 项目信息分组：主页 / 仓库 / 反馈外链行（数据驱动），零 props。
  import GithubIcon from "#assets/icons/github-icon.svelte";
  import { Button } from "#components/shadcn-svelte/button";
  import CardRow from "#components/common/card-row.svelte";
  import CardSection from "#components/common/card-section.svelte";
  import { m } from "#libs/i18n/paraglide/messages";
  import { toast } from "#libs/utils/toast";
  import { normalizeExternalUrl, openExternal } from "#libs/utils/opener";

  /** 外链行配置：文案键 + 地址；`git+` 是 npm 的传输前缀，展示与打开统一用还原后的地址 */
  const links: { label: string; url: string }[] = [
    { label: m.about_homepage(), url: __APP_PKG__.homepage },
    { label: m.about_repository(), url: __APP_PKG__.repository.url },
    { label: m.about_feedback(), url: __APP_PKG__.bugs.url },
  ].map((link) => ({ ...link, url: normalizeExternalUrl(link.url) }));

  /** 外链经 opener 打开；浏览器预览等无运行时按失败提示 */
  async function handleOpenLink(url: string): Promise<void> {
    if (!(await openExternal(url))) toast.error(m.about_open_failed());
  }
</script>

<CardSection title={m.about_project_info_title()}>
  {#each links as link (link.url)}
    <CardRow label={link.label} value={link.url}>
      {#snippet control()}
        <Button variant="outline" onclick={() => void handleOpenLink(link.url)}>
          <GithubIcon />
          {m.about_open()}
        </Button>
      {/snippet}
    </CardRow>
  {/each}
</CardSection>
