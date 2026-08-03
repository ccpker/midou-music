import type { ReactNode } from "react"
import { cn } from "../../lib/utils"

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  icon?: ReactNode
}

export function Input({ className, icon, ...props }: InputProps) {
  return (
    <div className="relative">
      {icon && (
        <div className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-400">
          {icon}
        </div>
      )}
      <input
        className={cn(
          "w-full h-10 rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-sm",
          "dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-100",
          "focus:outline-none focus:ring-2 focus:ring-zinc-400",
          "placeholder:text-zinc-400",
          icon && "pl-10",
          className,
        )}
        {...props}
      />
    </div>
  )
}

interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  options: { value: string; label: string }[]
  size?: "sm" | "default"
}

export function Select({ className, options, size = "default", ...props }: SelectProps) {
  return (
    <select
      className={cn(
        "rounded-lg border border-zinc-200 bg-zinc-50 px-3 text-sm cursor-pointer",
        "dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-100",
        "focus:outline-none focus:ring-2 focus:ring-zinc-400",
        size === "sm" ? "h-8" : "h-10",
        className,
      )}
      {...props}
    >
      {options.map(o => (
        <option key={o.value} value={o.value}>{o.label}</option>
      ))}
    </select>
  )
}
