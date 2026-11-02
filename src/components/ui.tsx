import {
  useEffect,
  useState,
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  type SelectHTMLAttributes,
  type TextareaHTMLAttributes,
} from "react";
import { AlertTriangle, CheckCircle2, Info, Inbox, Loader2, X } from "lucide-react";

export function cx(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}

type ButtonVariant = "primary" | "default" | "ghost" | "danger";

export function Button({
  variant = "default",
  size = "md",
  icon,
  children,
  className,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: "sm" | "md";
  icon?: ReactNode;
}) {
  const variantClass =
    variant === "primary"
      ? "btn-primary"
      : variant === "ghost"
        ? "btn-ghost"
        : variant === "danger"
          ? "btn-danger"
          : "";
  return (
    <button
      {...rest}
      className={cx("btn", variantClass, size === "sm" && "btn-sm", className)}
    >
      {icon}
      {children}
    </button>
  );
}

export function Card({
  children,
  className,
  padded = true,
}: {
  children: ReactNode;
  className?: string;
  padded?: boolean;
}) {
  return <div className={cx("card", padded && "card-pad", className)}>{children}</div>;
}

export function PanelTitle({
  title,
  action,
}: {
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="mb-3 flex items-center justify-between gap-3">
      <h2 className="panel-title">{title}</h2>
      {action}
    </div>
  );
}

export function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  return (
    <label className="block">
      <span className="label">{label}</span>
      {children}
      {hint ? <span className="mt-1 block text-[11.5px] text-muted">{hint}</span> : null}
    </label>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={cx("input", props.className)} />;
}

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} className={cx("textarea", props.className)} />;
}

export function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select {...props} className={cx("select", props.className)} />;
}

export function Badge({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "ok" | "warn" | "danger" | "info";
}) {
  const style: Record<string, string> = {
    neutral: "",
    accent: "border-[color-mix(in_srgb,var(--accent)_40%,transparent)] text-accent bg-accentsoft",
    ok: "border-[color-mix(in_srgb,var(--ok)_40%,transparent)] text-ok",
    warn: "border-[color-mix(in_srgb,var(--warn)_40%,transparent)] text-warn",
    danger: "border-[color-mix(in_srgb,var(--danger)_40%,transparent)] text-danger",
    info: "border-[color-mix(in_srgb,var(--info)_40%,transparent)] text-info",
  };
  return <span className={cx("badge", style[tone])}>{children}</span>;
}

export function ProgressBar({ value, className }: { value: number; className?: string }) {
  const clamped = Math.max(0, Math.min(100, value));
  return (
    <div className={cx("h-1.5 w-full overflow-hidden rounded-full bg-bg", className)}>
      <div
        className="h-full rounded-full bg-accent transition-[width] duration-300"
        style={{ width: `${clamped}%` }}
      />
    </div>
  );
}

export function StatCard({
  label,
  value,
  hint,
  icon,
}: {
  label: string;
  value: string | number;
  hint?: string;
  icon?: ReactNode;
}) {
  return (
    <div className="card px-4 py-3.5">
      <div className="flex items-start justify-between gap-2">
        <span className="text-[11.5px] font-semibold uppercase tracking-[0.04em] text-muted">
          {label}
        </span>
        <span className="text-muted opacity-70">{icon}</span>
      </div>
      <div className="kpi-value mt-2">{value}</div>
      {hint ? <div className="mt-0.5 text-[11.5px] text-muted">{hint}</div> : null}
    </div>
  );
}

export function EmptyState({
  title,
  description,
  action,
  icon,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-12 text-center">
      <div className="mb-1 text-muted opacity-60">{icon ?? <Inbox size={26} />}</div>
      <div className="text-sm font-semibold text-ink">{title}</div>
      {description ? (
        <p className="max-w-[420px] text-[12.5px] leading-relaxed text-muted">{description}</p>
      ) : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}

export function Loading({ label = "Loading" }: { label?: string }) {
  return (
    <div className="flex items-center justify-center gap-2 py-10 text-[12.5px] text-muted">
      <Loader2 size={15} className="animate-spin" />
      {label}…
    </div>
  );
}

export function ErrorNote({ message }: { message: string }) {
  return (
    <div className="flex items-start gap-2 rounded border border-[color-mix(in_srgb,var(--danger)_40%,transparent)] bg-[color-mix(in_srgb,var(--danger)_10%,transparent)] px-3 py-2 text-[12.5px] text-ink">
      <AlertTriangle size={15} className="mt-0.5 shrink-0 text-danger" />
      <span>{message}</span>
    </div>
  );
}

export function Modal({
  open,
  title,
  onClose,
  children,
  footer,
  width = 560,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/55 p-6 backdrop-blur-[1px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className="card my-10 w-full"
        style={{ maxWidth: width }}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="flex items-center justify-between border-b border-line px-4 py-3">
          <h3 className="text-[13.5px] font-semibold text-ink">{title}</h3>
          <Button variant="ghost" size="sm" onClick={onClose} aria-label="Close">
            <X size={15} />
          </Button>
        </div>
        <div className="px-4 py-4">{children}</div>
        {footer ? (
          <div className="flex justify-end gap-2 border-t border-line px-4 py-3">{footer}</div>
        ) : null}
      </div>
    </div>
  );
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "Delete",
  danger = true,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Modal
      open={open}
      title={title}
      onClose={onCancel}
      width={440}
      footer={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant={danger ? "danger" : "primary"} onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p className="text-[13px] leading-relaxed text-inksoft">{message}</p>
    </Modal>
  );
}

export function Tabs<T extends string>({
  tabs,
  value,
  onChange,
}: {
  tabs: Array<{ id: T; label: string; count?: number }>;
  value: T;
  onChange: (id: T) => void;
}) {
  return (
    <div className="flex gap-1 overflow-x-auto border-b border-line">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          className={cx(
            "relative -mb-px whitespace-nowrap px-3 py-2 text-[12.5px] font-medium transition-colors",
            value === tab.id
              ? "border-b-2 border-accent text-ink"
              : "border-b-2 border-transparent text-muted hover:text-ink",
          )}
        >
          {tab.label}
          {typeof tab.count === "number" ? (
            <span className="ml-1.5 text-[11px] text-muted">{tab.count}</span>
          ) : null}
        </button>
      ))}
    </div>
  );
}

// ------------------------------------------------------------------ toasts

export type ToastTone = "ok" | "error" | "info";

type ToastPayload = { message: string; tone: ToastTone };

export function toast(message: string, tone: ToastTone = "info") {
  window.dispatchEvent(new CustomEvent<ToastPayload>("storyops:toast", { detail: { message, tone } }));
}

export function Toaster() {
  const [items, setItems] = useState<Array<ToastPayload & { id: number }>>([]);

  useEffect(() => {
    let counter = 0;
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<ToastPayload>).detail;
      counter += 1;
      const id = counter;
      setItems((prev) => [...prev, { ...detail, id }]);
      window.setTimeout(() => {
        setItems((prev) => prev.filter((item) => item.id !== id));
      }, 4200);
    };
    window.addEventListener("storyops:toast", handler);
    return () => window.removeEventListener("storyops:toast", handler);
  }, []);

  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[330px] flex-col gap-2">
      {items.map((item) => (
        <div
          key={item.id}
          className="pointer-events-auto flex items-start gap-2 rounded border border-line bg-elevated px-3 py-2 text-[12.5px] shadow-lg"
        >
          {item.tone === "ok" ? (
            <CheckCircle2 size={15} className="mt-0.5 shrink-0 text-ok" />
          ) : item.tone === "error" ? (
            <AlertTriangle size={15} className="mt-0.5 shrink-0 text-danger" />
          ) : (
            <Info size={15} className="mt-0.5 shrink-0 text-info" />
          )}
          <span className="leading-snug text-inksoft">{item.message}</span>
        </div>
      ))}
    </div>
  );
}

export function useAsyncAction() {
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    try {
      await fn();
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      setBusy(false);
    }
  };
  return { busy, run };
}
