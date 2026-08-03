import * as React from "react"
import { cn } from "../../lib/utils"

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "secondary" | "ghost" | "destructive"
  size?: "sm" | "default" | "lg" | "icon"
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "default", children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          "inline-flex items-center justify-center rounded-lg font-medium cursor-pointer",
          "transition-colors focus-visible:outline-none focus-visible:ring-2",
          "disabled:opacity-50 disabled:pointer-events-none",
          variant === "default" && "bg-zinc-900 text-zinc-50 hover:bg-zinc-800 dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-zinc-200",
          variant === "secondary" && "bg-zinc-100 text-zinc-900 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-50 dark:hover:bg-zinc-700",
          variant === "ghost" && "hover:bg-zinc-100 dark:hover:bg-zinc-800",
          variant === "destructive" && "bg-red-600 text-white hover:bg-red-700",
          size === "sm" && "h-8 px-3 text-xs",
          size === "default" && "h-10 px-4 py-2 text-sm",
          size === "lg" && "h-12 px-6 text-base",
          size === "icon" && "h-9 w-9",
          className,
        )}
        {...props}
      >
        {children}
      </button>
    )
  }
)
Button.displayName = "Button"

export { Button }
