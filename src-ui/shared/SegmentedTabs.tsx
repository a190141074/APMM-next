// 横向分段选择器使用手动激活：方向键移动焦点，Enter 或空格才改变平台。
import { useId, useRef } from "react";

export function SegmentedTabs({
  label,
  value,
  options,
  disabled = false,
  onChange,
}: {
  label: string;
  value: string;
  options: { value: string; label: string; detail?: string }[];
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const id = useId();
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  return (
    <div className="segmented-scroll">
      <div className="segmented-tabs" role="tablist" aria-label={label}>
        {options.map((option, index) => (
          <button
            key={option.value}
            ref={(el) => {
              refs.current[index] = el;
            }}
            id={`${id}-${option.value}`}
            type="button"
            role="tab"
            aria-selected={value === option.value}
            tabIndex={value === option.value ? 0 : -1}
            disabled={disabled}
            onClick={() => onChange(option.value)}
            onKeyDown={(event) => {
              const next =
                event.key === "ArrowRight"
                  ? (index + 1) % options.length
                  : event.key === "ArrowLeft"
                    ? (index + options.length - 1) % options.length
                    : event.key === "Home"
                      ? 0
                      : event.key === "End"
                        ? options.length - 1
                        : -1;
              if (next >= 0) {
                event.preventDefault();
                refs.current[next]?.focus();
              }
            }}
          >
            <span>{option.label}</span>
            {option.detail && <small>{option.detail}</small>}
          </button>
        ))}
      </div>
    </div>
  );
}
