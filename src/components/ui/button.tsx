import { type ButtonHTMLAttributes } from 'react';

const base =
  'inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-[13px] font-medium transition-[background-color,border-color,box-shadow,opacity,transform] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500/30 active:translate-y-px disabled:pointer-events-none disabled:cursor-default disabled:opacity-40 dark:focus-visible:ring-zinc-400/30';

const variants = {
  default:
    'bg-zinc-900 text-white shadow-[inset_0_1px_rgba(255,255,255,0.18),0_1px_2px_rgba(0,0,0,0.08)] hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-white',
  outline:
    'border border-zinc-200 bg-white shadow-[0_1px_1px_rgba(0,0,0,0.03)] hover:bg-zinc-100 dark:border-zinc-700 dark:bg-zinc-900 dark:hover:bg-zinc-800',
  ghost: 'hover:bg-zinc-900/[0.06] dark:hover:bg-white/[0.08]',
};

const sizes = {
  default: 'h-8 px-3.5',
  sm: 'h-7 px-2.5 text-xs',
  compact: 'h-6 px-2 text-[11px]',
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
}

function Button({
  className,
  variant = 'default',
  size = 'default',
  ...props
}: ButtonProps) {
  return (
    <button
      className={[base, variants[variant], sizes[size], className]
        .filter(Boolean)
        .join(' ')}
      {...props}
    />
  );
}

export { Button };
