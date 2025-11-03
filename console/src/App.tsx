import {useEffect, useState} from 'react'
import {ServerStatus} from './ServerStatus'
import {LoginButton} from './LoginButton'
import {ToggleButton} from './ToggleButton'
import {StatusTable} from "./StatusTable";

// This app is delivered by the same Rust server that exposes the endpoints.
// In dev mode, requests are passed through a proxy, see vite.config.js.
const LOGIN_URL = '/authorize'
const TOGGLE_URL = '/toggle'
const STATUS_URL = '/status'

export const App = () => {
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const setDownloadState = (download_state: string) => {
        setStatus(Object.assign({}, status, { download_state }))
    }

    useEffect(() => {
        /*
        fetch(STATUS_URL)
            .then(response => response.json())
            .then(data => setStatus(data))
            .catch(error => console.log(error))
       */
        const es = new EventSource(STATUS_URL)
        es.onopen = () => console.log('SSE connection opened')
        es.onerror = (e) => console.warn('SSE error:', e)
        es.onmessage = (e) => setStatus(JSON.parse(e.data))
        return () => es.close();
    }, [])

    if (status == null) {
        return <b>Waiting for data from server...</b>
    }

    return (
        <div>
            <StatusTable status={status} />
            <LoginButton loginUrl={LOGIN_URL} authorized={ status.authorized } />
            <ToggleButton toggleUrl={TOGGLE_URL} disabled={ !status.authorized } downloadState={ status.download_state } setDownloadState={setDownloadState} />
        </div>
    )
}
