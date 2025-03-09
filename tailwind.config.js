module.exports = {
  content: ["./src/**/*.{html,ts}"],
  darkMode: 'media', // enables dark mode based on user's system preference
  theme: {
    extend: {
      borderRadius: {
        '2xl': '1rem',
        '3xl': '1.5rem',
      },
      colors: {
        // Apple-inspired colors
        'apple-blue': {
          DEFAULT: '#0071e3',
          dark: '#0077ed'
        },
        'apple-gray': {
          light: '#f5f5f7',
          DEFAULT: '#86868b',
          dark: '#1d1d1f'
        }
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          'San Francisco',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'sans-serif'
        ]
      }
    }
  },
  plugins: [
    require('@tailwindcss/forms')
  ]
}
