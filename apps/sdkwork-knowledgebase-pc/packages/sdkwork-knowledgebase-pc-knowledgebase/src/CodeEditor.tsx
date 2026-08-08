import { useEffect, useState } from 'react';
import Editor from '@monaco-editor/react';
import type { ReactKeyedComponentProps } from '@sdkwork/sdkwork-knowledgebase-pc-commons/reactKeyedProps';

export interface CodeEditorProps extends ReactKeyedComponentProps {
  initialContent: string;
  language?: string;
  onChange?: (content: string) => void;
}

export function CodeEditor({ initialContent, language = "typescript", onChange }: CodeEditorProps) {
  const [isDark, setIsDark] = useState(false);

  useEffect(() => {
    // Check if dark mode is active by checking the html tag
    const observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.attributeName === 'class') {
          setIsDark(document.documentElement.classList.contains('dark'));
        }
      });
    });

    observer.observe(document.documentElement, { attributes: true });
    setIsDark(document.documentElement.classList.contains('dark'));

    return () => observer.disconnect();
  }, []);

  return (
    <div className="absolute inset-0 bg-[var(--color-kb-editor)]">
      <Editor
        height="100%"
        language={language}
        value={initialContent}
        onChange={(val) => onChange && onChange(val || '')}
        theme={isDark ? "vs-dark" : "vs"}
        options={{
          minimap: { enabled: true, scale: 0.75 },
          fontSize: 14,
          fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, 'Courier New', monospace",
          fontLigatures: true,
          padding: { top: 16, bottom: 16 },
          scrollBeyondLastLine: false,
          smoothScrolling: true,
          cursorBlinking: "smooth",
          cursorSmoothCaretAnimation: "on",
          formatOnPaste: true,
          wordWrap: "on",
          bracketPairColorization: { enabled: true },
          guides: { bracketPairs: true, indentation: true },
          renderLineHighlight: "all",
          scrollbar: {
            verticalScrollbarSize: 10,
            horizontalScrollbarSize: 10,
          },
          lineHeight: 24,
          tabSize: 2,
        }}
      />
    </div>
  );
}
