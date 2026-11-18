// 作品资料与标签编辑；保留署名换行，公开链接在离开输入框时清理空项。
import { useState } from "react";
import type { AuthorMetadata } from "../../core/ipc/generated/AuthorMetadata";

export function AuthorMetadataFields({
  value,
  onChange,
}: {
  value: AuthorMetadata;
  onChange: (next: AuthorMetadata) => void;
}) {
  const [tag, setTag] = useState("");
  const [error, setError] = useState("");
  const fields = [
    ["title", "名称", false, 120],
    ["author", "作者显示名", true, 120],
    ["artist", "画师", true, 120],
    ["version", "作品版本", false, 64],
    ["gameVersion", "适用游戏版本", false, 256],
    ["category", "分类", false, 64],
    ["sourceUrl", "作品来源链接", false, 2048],
    ["changeNote", "更新说明", true, 16000],
    ["description", "简介", true, 10000],
  ] as const;
  const addTag = () => {
    const incoming = tag
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    const all = [...value.tags, ...incoming];
    if (
      all.length > 16 ||
      all.some((t) => [...t].length > 24) ||
      new Set(all.map((t) => t.toLowerCase())).size !== all.length
    ) {
      setError("最多 16 个标签，每个最多 24 字，且不能重复。");
      return;
    }
    onChange({ ...value, tags: all });
    setTag("");
    setError("");
  };
  return (
    <div className="author-fields">
      {fields.map(([key, label, multiline, maxLength]) => (
        <label
          className={`field ${key === "description" ? "wide-field" : ""}`}
          key={key}
        >
          {label}
          {["title", "version", "description"].includes(key) ? " *" : ""}
          {multiline ? (
            <textarea
              aria-label={`${label}${["title", "version", "description"].includes(key) ? " *" : ""}`}
              className={
                key === "author" || key === "artist"
                  ? "author-credit-input"
                  : undefined
              }
              maxLength={maxLength}
              value={value[key] || ""}
              onChange={(e) => onChange({ ...value, [key]: e.target.value })}
            />
          ) : (
            <input
              aria-label={`${label}${["title", "version", "description"].includes(key) ? " *" : ""}`}
              maxLength={maxLength}
              value={value[key] || ""}
              onChange={(e) => onChange({ ...value, [key]: e.target.value })}
              onBlur={
                key === "sourceUrl"
                  ? (e) =>
                      onChange({ ...value, sourceUrl: e.target.value.trim() })
                  : undefined
              }
            />
          )}
        </label>
      ))}
      {(
        [
          ["authorUrls", "作者主页链接"],
          ["artistUrls", "画师主页链接"],
        ] as const
      ).map(([key, label]) => (
        <label className="field" key={key}>
          {label}
          <textarea
            aria-label={label}
            placeholder="每行一个 HTTPS 链接"
            value={(value[key] || []).join("\n")}
            onChange={(e) =>
              onChange({ ...value, [key]: e.target.value.split("\n") })
            }
            onBlur={(e) =>
              onChange({
                ...value,
                [key]: e.target.value
                  .split("\n")
                  .map((url) => url.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
      ))}
      <div className="author-tag-editor">
        <label className="field" htmlFor="author-tag-input">
          标签
        </label>
        <div className="author-tag-input">
          <input
            id="author-tag-input"
            placeholder="输入标签，按回车添加"
            value={tag}
            onChange={(e) => {
              setTag(e.target.value);
              setError("");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                e.preventDefault();
                addTag();
              }
            }}
          />
          <button
            className="button"
            type="button"
            onClick={addTag}
            disabled={!tag.trim()}
          >
            添加标签
          </button>
        </div>
        <div className="work-tags">
          {value.tags.map((name, i) => (
            <span className="work-tag" key={`${name}-${i}`}>
              {name}
              <button
                type="button"
                aria-label={`移除标签 ${name}`}
                onClick={() =>
                  onChange({
                    ...value,
                    tags: value.tags.filter((_, n) => n !== i),
                  })
                }
              >
                ×
              </button>
            </span>
          ))}
        </div>
        {error && <p role="alert">{error}</p>}
      </div>
    </div>
  );
}
