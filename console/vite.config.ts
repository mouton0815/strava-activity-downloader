import react from '@vitejs/plugin-react-swc'

// https://vitejs.dev/config/
export default {
  base: "/console",
  plugins: [react()],
  server: {
    port: 2020
  }
}
