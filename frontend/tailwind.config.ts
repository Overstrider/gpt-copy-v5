import type { Config } from "tailwindcss";
import forms from "@tailwindcss/forms";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}", "./lib/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#171717",
        panel: "#f4f0e8",
        line: "#d8d0c1",
        accent: "#0f766e",
        signal: "#b45309"
      },
      boxShadow: {
        soft: "0 16px 50px rgba(23, 23, 23, 0.09)"
      }
    }
  },
  plugins: [forms]
};

export default config;
