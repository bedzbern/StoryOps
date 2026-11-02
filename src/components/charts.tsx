import { useId } from "react";
import type { LabeledValue, SeriesPoint } from "../lib/types";

function niceMax(value: number): number {
  if (value <= 0) return 1;
  const exponent = Math.floor(Math.log10(value));
  const base = Math.pow(10, exponent);
  const scaled = value / base;
  const step = scaled <= 1 ? 1 : scaled <= 2 ? 2 : scaled <= 5 ? 5 : 10;
  return step * base;
}

export function LineChart({
  points,
  height = 150,
  label,
}: {
  points: SeriesPoint[];
  height?: number;
  label?: string;
}) {
  const gradientId = useId();
  const width = 620;
  const padX = 8;
  const padY = 12;
  const max = niceMax(Math.max(...points.map((p) => p.value), 0));
  const stepX = points.length > 1 ? (width - padX * 2) / (points.length - 1) : 0;
  const y = (value: number) => height - padY - (value / max) * (height - padY * 2);
  const coords = points.map((p, i) => ({ x: padX + i * stepX, y: y(p.value), point: p }));
  const path = coords.map((c, i) => `${i === 0 ? "M" : "L"}${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(" ");
  const area =
    coords.length > 0
      ? `${path} L${coords[coords.length - 1].x.toFixed(1)},${height - padY} L${coords[0].x.toFixed(1)},${height - padY} Z`
      : "";

  return (
    <div>
      {label ? <div className="mb-1 text-[11px] uppercase tracking-wide text-muted">{label}</div> : null}
      <svg viewBox={`0 0 ${width} ${height}`} className="h-auto w-full" role="img" aria-label={label ?? "chart"}>
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.28" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0" />
          </linearGradient>
        </defs>
        {[0, 0.5, 1].map((ratio) => (
          <line
            key={ratio}
            x1={padX}
            x2={width - padX}
            y1={padY + ratio * (height - padY * 2)}
            y2={padY + ratio * (height - padY * 2)}
            stroke="var(--border)"
            strokeDasharray="2 4"
          />
        ))}
        {area ? <path d={area} fill={`url(#${gradientId})`} /> : null}
        {path ? <path d={path} fill="none" stroke="var(--accent)" strokeWidth={1.8} /> : null}
        {coords.map((c) => (
          <circle key={c.point.date} cx={c.x} cy={c.y} r={2.2} fill="var(--accent)">
            <title>{`${c.point.date}: ${Math.round(c.point.value).toLocaleString()}`}</title>
          </circle>
        ))}
      </svg>
      <div className="mt-1 flex justify-between text-[10.5px] text-muted">
        <span>{points[0]?.date ?? ""}</span>
        <span>{points[points.length - 1]?.date ?? ""}</span>
      </div>
    </div>
  );
}

export function BarList({
  items,
  unit = "",
  emptyLabel = "No data recorded yet",
}: {
  items: LabeledValue[];
  unit?: string;
  emptyLabel?: string;
}) {
  const max = Math.max(...items.map((i) => i.value), 0);
  if (items.length === 0 || max === 0) {
    return <div className="py-6 text-center text-[12px] text-muted">{emptyLabel}</div>;
  }
  return (
    <div className="flex flex-col gap-2">
      {items.map((item) => (
        <div key={item.label}>
          <div className="mb-1 flex items-baseline justify-between gap-2 text-[12px]">
            <span className="truncate text-inksoft">{item.label}</span>
            <span className="tabular-nums text-muted">
              {item.value.toLocaleString()}
              {unit}
            </span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-bg">
            <div
              className="h-full rounded-full bg-accent"
              style={{ width: `${Math.max(2, (item.value / max) * 100)}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

export function Donut({
  value,
  total,
  caption,
  size = 116,
}: {
  value: number;
  total: number;
  caption: string;
  size?: number;
}) {
  const ratio = total > 0 ? Math.min(1, value / total) : 0;
  const radius = size / 2 - 9;
  const circumference = 2 * Math.PI * radius;
  return (
    <div className="flex items-center gap-4">
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={caption}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--border)"
          strokeWidth={9}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--accent)"
          strokeWidth={9}
          strokeLinecap="round"
          strokeDasharray={`${circumference * ratio} ${circumference}`}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
        <text
          x="50%"
          y="50%"
          textAnchor="middle"
          dominantBaseline="central"
          fill="var(--text)"
          fontSize="17"
          fontWeight="600"
        >
          {Math.round(ratio * 100)}%
        </text>
      </svg>
      <div>
        <div className="text-[12.5px] text-inksoft">{caption}</div>
        <div className="text-[11.5px] text-muted">
          {value.toLocaleString()} of {total.toLocaleString()}
        </div>
      </div>
    </div>
  );
}

export function MiniBars({ values }: { values: number[] }) {
  const max = Math.max(...values, 1);
  return (
    <div className="flex h-10 items-end gap-[3px]">
      {values.map((value, index) => (
        <div
          key={index}
          className="w-full rounded-sm bg-accent/70"
          style={{ height: `${Math.max(4, (value / max) * 100)}%` }}
        />
      ))}
    </div>
  );
}
