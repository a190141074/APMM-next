// 颜色输入保留临时文本，合法后才提交；拖动始终只更新当前角色。
import { useEffect, useState, type PointerEvent } from "react";
import { Pipette } from "lucide-react";
import {
  cmykToRgb,
  hexToRgb,
  hsvToHex,
  rgbToCmyk,
  rgbToHex,
  rgbToHsv,
  validHex,
} from "../core/domain/appearance";
import { pickScreenColor } from "../core/services/appearance";

export function ColorControl({
  value,
  label,
  onChange,
  onPicking,
}: {
  value: string;
  label: string;
  onChange: (hex: string) => void;
  onPicking: (picking: boolean) => void;
}) {
  const [hex, setHex] = useState(value),
    [format, setFormat] = useState<"RGB" | "CMYK">("RGB");
  const [error, setError] = useState(""),
    [picking, setPicking] = useState(false);
  const [hue, setHue] = useState(() => rgbToHsv(hexToRgb(value))[0]);
  useEffect(() => {
    setHex(value);
    const hsv = rgbToHsv(hexToRgb(value));
    if (hsv[1] > 0) setHue(hsv[0]);
  }, [value]);
  const rgb = hexToRgb(value),
    [, saturation, brightness] = rgbToHsv(rgb);
  const values =
    format === "RGB" ? rgb : rgbToCmyk(rgb).map((n) => Math.round(n * 10) / 10);
  const palette = (e: PointerEvent<HTMLDivElement>) => {
    const box = e.currentTarget.getBoundingClientRect();
    onChange(
      hsvToHex(
        hue,
        Math.max(0, Math.min(1, (e.clientX - box.left) / box.width)),
        1 - Math.max(0, Math.min(1, (e.clientY - box.top) / box.height)),
      ),
    );
  };
  return (
    <div className="color-control">
      <div className="color-control-heading">
        <strong>{label}</strong>
        <span className="color-chip" style={{ background: value }} />
      </div>
      <div
        className="color-plane"
        style={{ backgroundColor: hsvToHex(hue, 1, 1) }}
        aria-label="饱和度与明度调色盘"
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          palette(e);
        }}
        onPointerMove={(e) => {
          if (e.currentTarget.hasPointerCapture(e.pointerId)) palette(e);
        }}
        onPointerUp={(e) => e.currentTarget.releasePointerCapture(e.pointerId)}
      >
        <i
          style={{
            left: `${saturation * 100}%`,
            top: `${(1 - brightness) * 100}%`,
          }}
        />
      </div>
      <label className="color-range">
        色相
        <input
          aria-label="色相"
          className="hue-range"
          type="range"
          min="0"
          max="359"
          value={hue}
          onChange={(e) => {
            const h = Number(e.target.value);
            setHue(h);
            onChange(hsvToHex(h, saturation, brightness));
          }}
        />
      </label>
      <div className="color-sliders">
        <label>
          饱和度
          <input
            aria-label="饱和度"
            type="range"
            min="0"
            max="100"
            value={saturation * 100}
            onChange={(e) =>
              onChange(hsvToHex(hue, Number(e.target.value) / 100, brightness))
            }
          />
        </label>
        <label>
          明度
          <input
            aria-label="明度"
            type="range"
            min="0"
            max="100"
            value={brightness * 100}
            onChange={(e) =>
              onChange(hsvToHex(hue, saturation, Number(e.target.value) / 100))
            }
          />
        </label>
      </div>
      <label className="color-hex">
        HEX
        <input
          aria-label="HEX"
          value={hex}
          maxLength={7}
          spellCheck={false}
          aria-invalid={!validHex(hex)}
          onChange={(e) => {
            setHex(e.target.value);
            if (validHex(e.target.value))
              onChange(e.target.value.toLowerCase());
          }}
          onBlur={() => setHex(value)}
        />
      </label>
      <div className="action-row">
        {(["RGB", "CMYK"] as const).map((f) => (
          <button
            type="button"
            className={`button ${format === f ? "primary" : ""}`}
            key={f}
            onClick={() => setFormat(f)}
          >
            {f}
          </button>
        ))}
        <button
          type="button"
          className="button"
          disabled={picking}
          onClick={async () => {
            setError("");
            setPicking(true);
            onPicking(true);
            try {
              const color = await pickScreenColor();
              if (color) onChange(color);
            } catch (e) {
              setError(String(e));
            } finally {
              setPicking(false);
              onPicking(false);
            }
          }}
        >
          <Pipette size={14} />
          吸色
        </button>
      </div>
      <div className="color-numbers">
        {values.map((n, index) => (
          <label key={`${format}-${index}`}>
            {format[index]}
            {format === "CMYK" ? " %" : ""}
            <input
              aria-label={`${format[index]} 数值`}
              type="number"
              min="0"
              max={format === "RGB" ? 255 : 100}
              step={format === "RGB" ? 1 : 0.1}
              value={n}
              onChange={(e) => {
                if (e.target.value === "") return;
                const next = [...values];
                next[index] = Number(e.target.value);
                onChange(rgbToHex(format === "RGB" ? next : cmykToRgb(next)));
              }}
            />
          </label>
        ))}
      </div>
      <p className="muted">
        {picking
          ? "点击屏幕任意位置取色，Esc 或右键取消。"
          : format === "CMYK"
            ? "CMYK 用于便捷换算，颜色按屏幕 sRGB 保存。"
            : "支持数值输入、调色盘与屏幕取色。"}
      </p>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
