// 候选作者、画师与多链接编辑保持紧凑，所有外链仍由用户通过 HTTPS 入口点击打开。
import { useState } from "react";
import { ExternalLink, Plus, X } from "lucide-react";
import type { CandidateInfo } from "../core/ipc/generated/CandidateInfo";
function CreditLinks({
  label,
  links,
  onChange,
  onOpen,
  disabled,
}: {
  label: string;
  links: string[];
  onChange: (links: string[]) => void;
  onOpen: (url: string) => void;
  disabled: boolean;
}) {
  return (
    <div className="credit-link-editor">
      {links.map((url, index) => (
        <div className="credit-link-input" key={index}>
          <input
            aria-label={`${label} ${index + 1}`}
            placeholder="https://"
            value={url}
            disabled={disabled}
            onChange={(e) =>
              onChange(
                links.map((value, i) => (i === index ? e.target.value : value)),
              )
            }
          />
          <button
            type="button"
            className="icon-button"
            aria-label={`打开${label} ${index + 1}`}
            disabled={disabled || !url}
            onClick={() => onOpen(url)}
          >
            <ExternalLink size={14} />
          </button>
          <button
            type="button"
            className="icon-button"
            aria-label={`移除${label} ${index + 1}`}
            disabled={disabled}
            onClick={() => onChange(links.filter((_, i) => i !== index))}
          >
            <X size={14} />
          </button>
        </div>
      ))}
      <button
        type="button"
        className="text-button"
        disabled={disabled}
        onClick={() => onChange([...links, ""])}
      >
        <Plus size={13} />
        添加{label}
      </button>
    </div>
  );
}
export function CandidateCreditsFields({
  info,
  onChange,
  onOpen,
  disabled,
}: {
  info: CandidateInfo;
  onChange: (info: CandidateInfo) => void;
  onOpen: (url: string) => void;
  disabled: boolean;
}) {
  const [authorOpen, setAuthorOpen] = useState(
    info.provenance.authorUrls.length > 0,
  );
  const [artistOpen, setArtistOpen] = useState(
    !!info.provenance.artist || info.provenance.artistUrls.length > 0,
  );
  const change = (patch: Partial<CandidateInfo["provenance"]>) =>
    onChange({ ...info, provenance: { ...info.provenance, ...patch } });
  return (
    <div className="mod-credit-fields">
      <div className="mod-info-fields">
        <label className="field">
          作者
          <input
            value={info.provenance.author}
            onChange={(e) => change({ author: e.target.value })}
            disabled={disabled}
          />
        </label>
        <label className="field">
          版本
          <input
            value={info.provenance.version}
            onChange={(e) => change({ version: e.target.value })}
            disabled={disabled}
          />
        </label>
        <label className="field source-link-field">
          来源链接（HTTPS）
          <span className="credit-link-input">
            <input
              value={info.provenance.sourceUrl}
              onChange={(e) => change({ sourceUrl: e.target.value })}
              disabled={disabled}
            />
            {info.provenance.sourceUrl && (
              <button
                type="button"
                className="icon-button"
                aria-label="打开来源链接"
                disabled={disabled}
                onClick={() => onOpen(info.provenance.sourceUrl)}
              >
                <ExternalLink size={14} />
              </button>
            )}
          </span>
        </label>
      </div>
      <details
        className="mod-credit-details"
        open={authorOpen}
        onToggle={(e) => setAuthorOpen(e.currentTarget.open)}
      >
        <summary>
          作者链接
          {info.provenance.authorUrls.length
            ? `（${info.provenance.authorUrls.length}）`
            : "（可选）"}
        </summary>
        <CreditLinks
          label="作者链接"
          links={info.provenance.authorUrls}
          onChange={(authorUrls) => change({ authorUrls })}
          onOpen={onOpen}
          disabled={disabled}
        />
      </details>
      <details
        className="mod-credit-details"
        open={artistOpen}
        onToggle={(e) => setArtistOpen(e.currentTarget.open)}
      >
        <summary>画师信息（可选）</summary>
        <label className="field">
          画师
          <input
            value={info.provenance.artist}
            onChange={(e) => change({ artist: e.target.value })}
            disabled={disabled}
          />
        </label>
        <CreditLinks
          label="画师链接"
          links={info.provenance.artistUrls}
          onChange={(artistUrls) => change({ artistUrls })}
          onOpen={onOpen}
          disabled={disabled}
        />
      </details>
      <label className="field">
        备注（仅本机）
        <textarea
          rows={2}
          value={info.note}
          onChange={(e) => onChange({ ...info, note: e.target.value })}
          disabled={disabled}
        />
      </label>
    </div>
  );
}
