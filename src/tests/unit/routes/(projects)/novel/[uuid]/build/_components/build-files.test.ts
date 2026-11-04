import { describe, expect, it } from "vitest";
import {
  firstFilePath,
  isImageFile,
  langOf,
} from "../../../../../../../../routes/(projects)/novel/[uuid]/build/_components/build-files";
import type { BuildFileNode } from "../../../../../../../../libs/commands/types";

const dir = (name: string, path: string, children: BuildFileNode[]): BuildFileNode => ({
  name,
  path,
  editable: false,
  children,
});
const file = (name: string, path: string, editable = true): BuildFileNode => ({
  name,
  path,
  editable,
  children: null,
});

describe("构建文件辅助", () => {
  it("按扩展名判定语言", () => {
    expect(langOf("EPUB/text/chapter-001.xhtml")).toBe("xml");
    expect(langOf("EPUB/content.opf")).toBe("xml");
    expect(langOf("EPUB/styles/base.css")).toBe("css");
    expect(langOf("mimetype")).toBe("text");
    expect(langOf("EPUB/images/cover.jpg")).toBe("text");
  });

  it("深度优先找首个文件", () => {
    const nodes = [
      dir("EPUB", "EPUB", [
        dir("text", "EPUB/text", [file("chapter-001.xhtml", "EPUB/text/chapter-001.xhtml")]),
        file("nav.xhtml", "EPUB/nav.xhtml"),
      ]),
      file("mimetype", "mimetype", false),
    ];
    expect(firstFilePath(nodes)).toBe("EPUB/text/chapter-001.xhtml");
    expect(firstFilePath([])).toBeNull();
  });

  it("识别图片文件", () => {
    expect(isImageFile("EPUB/images/cover.jpg")).toBe(true);
    expect(isImageFile("EPUB/images/cover.webp")).toBe(true);
    expect(isImageFile("EPUB/content.opf")).toBe(false);
  });
});
