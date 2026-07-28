interface OutputTextProps {
  text: string;
  error?: boolean;
  className?: string;
}

export function OutputText({
  text,
  error = false,
  className = '',
}: OutputTextProps) {
  return (
    <pre
      className={`font-mono text-[12px] leading-5 break-words whitespace-pre-wrap select-text ${
        error
          ? 'text-red-700 dark:text-red-300'
          : 'text-zinc-800 dark:text-zinc-200'
      } ${className}`}
    >
      {text}
    </pre>
  );
}
