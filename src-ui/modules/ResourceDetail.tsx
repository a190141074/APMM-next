// 详情页保留皮肤、部位和整组操作；候选行与直接列表使用相同的状态和命令。
import { useEffect, useState } from "react";
import { ArrowLeft } from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import {
  variantGroups,
  filterResources,
  labels,
  type Route,
  type Filter,
  type SearchMode,
} from "../core/domain/resources";
import {
  resourceSizeStyle,
  type ResourceSize,
} from "../core/domain/resourcePresentation";
import { Badge, Empty } from "../shared/Ui";
import { useResourceActions } from "../hooks/useResourceActions";
import { ResourceCandidateRow } from "./ResourceCandidateRow";
export function ResourceDetail({
  resources,
  onClose,
  focusKey,
  filter = "all",
  section = "",
  search = "",
  searchMode = "smart",
  size = "medium",
  onClearFilters,
}: {
  resources: ResourceRecord[];
  onClose: () => void;
  focusKey?: string;
  filter?: Filter;
  section?: string;
  search?: string;
  searchMode?: SearchMode;
  size?: ResourceSize;
  onClearFilters?: () => void;
}) {
  const [byPart, setByPart] = useState(false);
  const actions = useResourceActions(resources);
  const { choices, apply, pending, notice } = actions;
  const first = resources[0];
  const paired = ["lands", "cursors"].includes(first.category);
  const visible = filterResources(
    resources,
    first.category as Route,
    search,
    filter,
    section,
    searchMode,
  );
  const groups = variantGroups(visible, byPart);
  useEffect(() => {
    if (focusKey)
      document
        .getElementById(`part-${focusKey}`)
        ?.scrollIntoView({ block: "center" });
  }, [focusKey, first.entityId]);
  return (
    <section
      className="detail-page"
      style={resourceSizeStyle(size)}
      aria-label={`${first.entityName}资源详情`}
    >
      <div className="detail-top">
        <button className="button subtle" onClick={onClose}>
          <ArrowLeft size={17} />
          返回列表
        </button>
        <span className="breadcrumb">
          {labels[first.category as Route]} /{" "}
          <strong>{first.entityName}</strong>
        </span>
        <div className="spacer" />
        <div className="segments">
          <button
            className={!byPart ? "active" : ""}
            onClick={() => setByPart(false)}
          >
            按皮肤
          </button>
          <button
            className={byPart ? "active" : ""}
            onClick={() => setByPart(true)}
          >
            按部位
          </button>
        </div>
      </div>
      <div className="detail-scroll">
        <div className="page-heading">
          <div>
            <h1>
              {first.entityName}{" "}
              <span className="heading-count">#{first.entityId}</span>
            </h1>
            <p>
              {visible.length} / {resources.length} 个资源 ·{" "}
              {visible.reduce((n, r) => n + r.files.length, 0)} 个
              Mod。点击候选即应用，点击原版还原。
            </p>
          </div>
          <Badge tone={pending ? "" : "success"}>
            {pending ? `${pending} 次切换处理中` : "与资源库同步"}
          </Badge>
        </div>
        {paired && (
          <div className="selectionbar">
            <strong>整组操作 · {resources.length}/2 个部位</strong>
            <button
              className="button primary"
              disabled={
                resources.length !== 2 ||
                resources.some((r) => !choices[r.key] && !r.activeHash)
              }
              onClick={() => void apply(resources)}
            >
              应用整组候选
            </button>
            <button
              className="button"
              onClick={() => void apply(resources, null)}
            >
              还原整组
            </button>
          </div>
        )}
        {!visible.length && (
          <Empty
            title="当前筛选没有匹配资源"
            detail="清除筛选以查看此组的全部资源。"
          />
        )}
        {(filter !== "all" || section || search) && onClearFilters && (
          <button className="button subtle" onClick={onClearFilters}>
            清除筛选
          </button>
        )}
        <nav className="skin-index" aria-label="皮肤定位">
          {groups.map((g) => (
            <button
              className="button subtle"
              key={g.key}
              onClick={() =>
                document
                  .getElementById(`skin-${g.key}`)
                  ?.scrollIntoView({ block: "start" })
              }
            >
              {g.label} <small>{g.rows.length}</small>
            </button>
          ))}
        </nav>
        {groups.map((g) => (
          <section className="skin-group" id={`skin-${g.key}`} key={g.key}>
            <div className="section-heading">
              <h2>{g.label}</h2>
              <span>{g.rows.length} 个部位</span>
            </div>
            {g.rows.map((r) => (
              <ResourceCandidateRow
                key={r.key}
                row={r}
                actions={actions}
                byPart={byPart}
                focusKey={focusKey}
              />
            ))}
          </section>
        ))}
      </div>
      {notice && (
        <div className="detail-notice" role="status">
          {notice}
        </div>
      )}
      {actions.dialogs}
    </section>
  );
}
