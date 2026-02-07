import daisyui from 'daisyui'

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './.vitepress/**/*.{js,ts,vue}',
    './zh/**/*.{md,vue}',
    './en/**/*.{md,vue}',
  ],
  theme: {
    extend: {},
  },
  plugins: [daisyui],
  daisyui: {
    themes: [
      {
        yaoxiang: {
          "primary": "#6366f1",
          "secondary": "#8b5cf6",
          "accent": "#06b6d4",
          "neutral": "#1f2937",
          "base-100": "#ffffff",
          "base-200": "#f8fafc",
          "base-300": "#e2e8f0",
          "info": "#3b82f6",
          "success": "#22c55e",
          "warning": "#f59e0b",
          "error": "#ef4444",
        },
      },
      "dark",
      "light",
    ],
  },
}
