import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'

const SERVER_URL = 'http://localhost:2525'

export default defineConfig({
    plugins: [react()],
    base: '/console',
    server: {
        port: 2020,
        proxy: {
            '/authorize': SERVER_URL,
            '/status': SERVER_URL,
            '/toggle': SERVER_URL
        }
    }
})
