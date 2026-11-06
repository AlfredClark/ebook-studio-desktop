import { describe, expect, it } from "vitest";
import {
  formatImageSize,
  isExpandedByDefault,
  isImageFile,
  langOf,
} from "../../../../../../../../routes/(projects)/comic/[uuid]/build/_components/build-files";

describe("漫画构建文件辅助", () => {
  it("按扩展名判定语言", () => {
    expect(langOf("EPUB/page/page-001.xhtml")).toBe("xml");
    expect(langOf("EPUB/content.opf")).toBe("xml");
    expect(langOf("EPUB/styles/base.css")).toBe("css");
    expect(langOf("mimetype")).toBe("text");
    expect(langOf("EPUB/images/page-001.jpg")).toBe("text");
  });

  it("识别图片文件", () => {
    expect(isImageFile("EPUB/images/page-001.jpg")).toBe(true);
    expect(isImageFile("EPUB/images/page-001.webp")).toBe(true);
    expect(isImageFile("EPUB/content.opf")).toBe(false);
    expect(isImageFile("EPUB/page/page-001.xhtml")).toBe(false);
  });

  it("图片大小按 B / KB / MB 分档", () => {
    expect(formatImageSize(512)).toBe("512 B");
    expect(formatImageSize(2048)).toBe("2.0 KB");
    expect(formatImageSize(3 * 1024 * 1024)).toBe("3.0 MB");
  });

  it("仅顶层 EPUB 与 META-INF 默认展开", () => {
    expect(isExpandedByDefault("EPUB")).toBe(true);
    expect(isExpandedByDefault("META-INF")).toBe(true);
    expect(isExpandedByDefault("EPUB/page")).toBe(false);
    expect(isExpandedByDefault("EPUB/images")).toBe(false);
    expect(isExpandedByDefault("mimetype")).toBe(false);
  });
});
