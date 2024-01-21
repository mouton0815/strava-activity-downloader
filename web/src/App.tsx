import {useEffect, useState} from 'react'
import {ServerStatus} from './Status'
import {LoginButton} from './LoginButton'
import {ToggleButton} from './ToggleButton'
import {StatusTable} from "./StatusTable";

const STATUS_URL = 'http://localhost:3000/status'

export const App = () => {
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const setSchedulerState = (scheduler_state: string) => {
        setStatus(Object.assign({}, status, { scheduler_state }))
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
        return <b>Loading ...</b>
    }

    return (
        <div>
            <StatusTable status={status} />
            <LoginButton authorized={ status.authorized } />
            <ToggleButton disabled={ !status.authorized } schedulerState={ status.scheduler_state } setSchedulerState={setSchedulerState} />
        </div>
    )
}
