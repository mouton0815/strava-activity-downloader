import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'

export default defineConfig({
    plugins: [react()],
    base: '/tilemap',
    server: {
        port: 2025,
        proxy: {
            '/tiles': {
                target: 'http://localhost:2525',
                changeOrigin: true
            }
        }
    }
})
