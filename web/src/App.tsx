import {useEffect, useState} from 'react'
import {ServerStatus} from './ServerStatus'
import {LoginButton} from './LoginButton'
import {ToggleButton} from './ToggleButton'
import {StatusTable} from "./StatusTable";

const SERVER_URL = 'http://localhost:2525' // Base URL of the Rust server
const LOGIN_URL = `${SERVER_URL}/authorize`
const TOGGLE_URL = `${SERVER_URL}/toggle`
const STATUS_URL = `${SERVER_URL}/status`

export const App = () => {
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const setDownloadState = (download_state: string) => {
        setStatus(Object.assign({}, status, { download_state }))
    }

    useEffect(() => {
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
