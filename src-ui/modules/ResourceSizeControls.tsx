import { Check, ChevronDown } from "lucide-react";
import * as Popover from "@radix-ui/react-popover";
import { useId, useRef, useState } from "react";
import {
  resourceSizeOptions,
  type ResourceSize,
} from "../core/domain/resourcePresentation";

/** 资源页和详情页共用的尺寸选择；统一开关只影响展示尺寸，不改变资源状态。 */
export function ResourceSizeControls({
  size,
  unified,
  onSizeChange,
  onUnifiedChange,
}: {
  size: ResourceSize;
  unified: boolean;
  onSizeChange: (size: ResourceSize) => void;
  onUnifiedChange: (unified: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const options = useRef<HTMLDivElement>(null);
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger
        className="select-trigger"
        role="combobox"
        aria-label="预览尺寸"
        aria-controls={id}
        aria-haspopup="dialog"
      >
        <span>
          {resourceSizeOptions.find((option) => option.value === size)?.label}
        </span>
        <ChevronDown size={14} />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className="select-content resource-size-popover"
          align="start"
          sideOffset={6}
          collisionPadding={12}
          onOpenAutoFocus={(event) => {
            event.preventDefault();
            options.current
              ?.querySelector<HTMLButtonElement>('[aria-selected="true"]')
              ?.focus();
          }}
        >
          <div
            ref={options}
            id={id}
            role="listbox"
            aria-label="预览尺寸档位"
            className="resource-size-options"
          >
            {resourceSizeOptions.map((option, index) => (
              <button
                key={option.value}
                type="button"
                role="option"
                aria-selected={size === option.value}
                tabIndex={size === option.value ? 0 : -1}
                className="select-option"
                onKeyDown={(event) => {
                  const direction =
                    event.key === "ArrowDown"
                      ? 1
                      : event.key === "ArrowUp"
                        ? -1
                        : 0;
                  if (!direction && event.key !== "Home" && event.key !== "End")
                    return;
                  event.preventDefault();
                  const next =
                    event.key === "Home"
                      ? 0
                      : event.key === "End"
                        ? resourceSizeOptions.length - 1
                        : (index + direction + resourceSizeOptions.length) %
                          resourceSizeOptions.length;
                  options.current
                    ?.querySelectorAll<HTMLButtonElement>('[role="option"]')
                    [next]?.focus();
                }}
                onClick={() => {
                  onSizeChange(option.value as ResourceSize);
                  setOpen(false);
                }}
              >
                {option.label}
                {size === option.value && <Check size={14} />}
              </button>
            ))}
          </div>
          <label className={`size-unified-toggle ${unified ? "active" : ""}`}>
            <input
              type="checkbox"
              aria-label="统一大小"
              checked={unified}
              onChange={(event) => onUnifiedChange(event.target.checked)}
            />
            <span>统一大小</span>
          </label>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
