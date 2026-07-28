import { cn } from '@/lib/utils';
import { type VariantProps, cva } from 'class-variance-authority';
import { type ButtonHTMLAttributes } from 'react';

const buttonVariants = cva(
  'inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-[13px] font-medium transition-[background-color,border-color,box-shadow,opacity,transform] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-500/30 active:translate-y-px disabled:pointer-events-none disabled:cursor-default disabled:opacity-40 dark:focus-visible:ring-zinc-400/30',
  {
    variants: {
      variant: {
        default:
          'bg-zinc-900 text-white shadow-[inset_0_1px_rgba(255,255,255,0.18),0_1px_2px_rgba(0,0,0,0.08)] hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-white',
        outline:
          'border border-zinc-200 bg-white shadow-[0_1px_1px_rgba(0,0,0,0.03)] hover:bg-zinc-100 dark:border-zinc-700 dark:bg-zinc-900 dark:hover:bg-zinc-800',
        ghost: 'hover:bg-zinc-900/[0.06] dark:hover:bg-white/[0.08]',
      },
      size: {
        default: 'h-8 px-3.5',
        sm: 'h-7 px-2.5 text-xs',
        lg: 'h-9 px-5',
        icon: 'size-7 p-0',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  }
);

interface ButtonProps
  extends
    ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

function Button({ className, variant, size, ...props }: ButtonProps) {
  return (
    <button
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
