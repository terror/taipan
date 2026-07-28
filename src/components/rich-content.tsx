interface RichContentProps {
  html: string;
  className?: string;
}

export function RichContent({ html, className = '' }: RichContentProps) {
  return (
    <div
      className={`rich-content select-text ${className}`}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
