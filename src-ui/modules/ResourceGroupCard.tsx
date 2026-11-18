// 角色紧凑卡只展示实体摘要，点击进入详情，候选应用仍由详情负责。
import { Check, ChevronRight } from "lucide-react";
import {
  resourceGroupSummary,
  type ResourceGroup,
  type Route,
} from "../core/domain/resources";
import { AssetImage, Badge } from "../shared/Ui";
export function ResourceGroupCard({
  group,
  route,
  compact = false,
  selected,
  onSelect,
  onOpen,
}: {
  group: ResourceGroup;
  route: Route;
  compact?: boolean;
  selected: boolean;
  onSelect: () => void;
  onOpen: () => void;
}) {
  const { cover: r, mods, applied } = resourceGroupSummary(group);
  const avatar = compact
    ? group.resources.find(
        (row) =>
          row.section === "item_icons" &&
          row.canonical === `UT_Item_Hero_${r.entityId}`,
      )
    : undefined;
  const stats = (
    <>
      <span>{mods ? `${mods} 个 Mod` : "无 Mod"}</span>
      {applied ? (
        <Badge tone="success">
          已应用{compact ? ` ${applied} 个部位` : ""}
        </Badge>
      ) : (
        <span className="muted">
          {route === "lands" || route === "cursors"
            ? `${group.resources.length}/2`
            : "原版"}
        </span>
      )}
    </>
  );
  return (
    <article
      data-resource-view-key={group.key}
      data-drop-resource-key={
        route !== "heroes" &&
        group.resources.length === 1 &&
        r.applicationSlot &&
        r.section !== "temporary"
          ? r.key
          : undefined
      }
      className={`resource-card ${compact ? "hero-compact-card" : ""} ${selected ? "selected" : ""}`}
    >
      <button
        className={`select-mark ${selected ? "checked" : ""}`}
        aria-label={`选择 ${r.entityName}`}
        aria-pressed={selected}
        onClick={onSelect}
      >
        {selected && <Check size={13} />}
      </button>
      <button className="resource-open" onClick={onOpen}>
        <AssetImage
          key={compact ? "avatar" : "cover"}
          thumbnail
          className={compact ? "hero-avatar" : undefined}
          path={
            compact ? avatar?.originalUrl : r.originalUrl || r.files[0]?.path
          }
          revision={!compact && !r.originalUrl ? r.files[0]?.hash : undefined}
          label={compact ? `${r.entityName}头像` : r.entityName}
          status={compact ? avatar?.originalStatus : r.originalStatus}
        />
        {compact ? (
          <div className="hero-compact-caption">
            <strong>{r.entityName}</strong>
            <ChevronRight size={13} aria-hidden="true" />
            <span className="hero-compact-meta">
              #{r.entityId} · {group.resources.length} 个部位
            </span>
            <div className="hero-compact-stats">{stats}</div>
          </div>
        ) : (
          <div className="resource-caption">
            <div>
              <strong>{r.entityName}</strong>
              <span>
                {route === "heroes" || route === "monsters"
                  ? `#${r.entityId} · ${group.resources.length} 个部位`
                  : r.canonical}
              </span>
            </div>
            <ChevronRight size={15} />
          </div>
        )}
      </button>
      {!compact && <div className="resource-footer">{stats}</div>}
    </article>
  );
}
