import { ArrowUpRight, Layers3 } from "lucide-react";
import { Button } from "@/components/ui/button";

export function App() {
  return (
    <main className="grid min-h-svh place-items-center bg-background p-6 text-foreground">
      <section className="w-full max-w-xl rounded-2xl border bg-card p-8 shadow-sm">
        <div className="mb-8 flex size-12 items-center justify-center rounded-xl bg-primary text-primary-foreground">
          <Layers3 className="size-6" aria-hidden="true" />
        </div>
        <p className="mb-2 text-sm font-medium text-muted-foreground">Desktop application</p>
        <h1 className="text-4xl font-semibold tracking-tight">Taipan</h1>
        <p className="mt-4 max-w-md leading-7 text-muted-foreground">
          A clean Tauri, React, and TypeScript foundation for your next desktop experience.
        </p>
        <Button className="mt-8" type="button">
          Start building
          <ArrowUpRight className="size-4" aria-hidden="true" />
        </Button>
      </section>
    </main>
  );
}
