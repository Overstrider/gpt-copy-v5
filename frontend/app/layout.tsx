import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "gpt-copy-v5",
  description: "ChatGPT-style interface backed by Axum and OpenRouter"
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
