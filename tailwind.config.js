/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./assets/views/**/*.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
      },
    },
  },
  plugins: [],
}
