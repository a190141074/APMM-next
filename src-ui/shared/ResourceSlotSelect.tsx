// 归属选择复用资源目录与鼠标、地块分组；只返回已经存在的完整槽位身份。
import { useMemo } from "react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import { labels, resourceGroupKey, type Route } from "../core/domain/resources";
import { ThemedSelect } from "./ThemedSelect";

export function ResourceSlotSelect({
  resources,
  value,
  onChange,
  allowTemporary = false,
  disabled = false,
}: {
  resources: ResourceRecord[];
  value: string;
  onChange: (row: ResourceRecord | null) => void;
  allowTemporary?: boolean;
  disabled?: boolean;
}) {
  const rows = useMemo(
    () => resources.filter((r) => r.category !== "temporary"),
    [resources],
  );
  const selected = rows.find((r) => r.key === value);
  const category = selected?.category || "";
  const object = selected ? resourceGroupKey(selected) : "";
  const objects = rows.filter((r) => r.category === category);
  const variants = objects.filter((r) => resourceGroupKey(r) === object);
  const parts = variants.filter((r) => r.variantKey === selected?.variantKey);
  const options = (
    rs: ResourceRecord[],
    key: (r: ResourceRecord) => string,
    label: (r: ResourceRecord) => string,
  ) => [
    ...new Map(
      rs.map((r) => [key(r), { value: key(r), label: label(r) }]),
    ).values(),
  ];
  return (
    <div className="slot-select">
      <ThemedSelect
        label="归属分类"
        disabled={disabled}
        value={category}
        onValueChange={(v) =>
          onChange(rows.find((r) => r.category === v) || null)
        }
        options={[
          {
            value: "",
            label: allowTemporary ? "临时" : "选择分类",
          },
          ...options(
            rows,
            (r) => r.category,
            (r) => labels[r.category as Route],
          ),
        ]}
      />
      {selected && (
        <>
          <ThemedSelect
            label="归属对象"
            disabled={disabled}
            value={object}
            onValueChange={(v) =>
              onChange(objects.find((r) => resourceGroupKey(r) === v) || null)
            }
            options={options(
              objects,
              (r) => resourceGroupKey(r),
              (r) =>
                ["lands", "cursors"].includes(r.category)
                  ? r.fullName
                  : r.entityName,
            )}
          />
          <ThemedSelect
            label="归属变体"
            disabled={disabled}
            value={selected.variantKey}
            onValueChange={(v) =>
              onChange(variants.find((r) => r.variantKey === v) || null)
            }
            options={options(
              variants,
              (r) => r.variantKey,
              (r) => r.variantLabel || "公共资源",
            )}
          />
          <ThemedSelect
            label="归属部位"
            disabled={disabled}
            value={value}
            onValueChange={(v) =>
              onChange(parts.find((r) => r.key === v) || null)
            }
            options={options(
              parts,
              (r) => r.key,
              (r) => `${r.resourceLabel || r.sectionLabel} · ${r.canonical}`,
            )}
          />
        </>
      )}
    </div>
  );
}
