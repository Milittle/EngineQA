import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#f2fbf6',
          100: '#dcf5e7',
          500: '#1f9d62',
          700: '#167548'
        }
      },
      lineClamp: {
        3: '3',
        4: '4',
        5: '5',
      },
    }
  },
  plugins: []
} satisfies Config;
