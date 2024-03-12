import {useEffect, useState} from 'react'
import {ServerStatus} from './ServerStatus'
import {LoginButton} from './LoginButton'
import {ToggleButton} from './ToggleButton'
import {StatusTable} from "./StatusTable";

const STATUS_URL = 'http://localhost:2525/status'

export const App = () => {
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const setDownloadState = (download_state: string) => {
        setStatus(Object.assign({}, status, { download_state }))
    }

    useEffect(() => {
        const es = new EventSource(STATUS_URL)
        es.onopen = () => console.log('SSE connection opened')
        es.onerror = (e) => console.warn('SSE error:', e)
        es.onmessage = (e) => {
            console.log('SSE:', e.data)
            setStatus(JSON.parse(e.data))
        }
        return () => es.close();
    }, [])

    if (status == null) {
        return <b>Waiting for data from server...</b>
    }

    return (
        <div>
            <StatusTable status={status} />
            <LoginButton authorized={ status.authorized } />
            <ToggleButton disabled={ !status.authorized } downloadState={ status.download_state } setDownloadState={setDownloadState} />
        </div>
    )
}
