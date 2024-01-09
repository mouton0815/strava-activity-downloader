import {useEffect, useState} from 'react'
import {ServerStatus} from './Status'
import {LoginButton} from './LoginButton'
import {ToggleButton} from './ToggleButton'
import {StatusTable} from "./StatusTable";

const STATUS_URL = 'http://localhost:3000/status'

export const App = () => {
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const fetchStatus = () => {
        fetch(STATUS_URL)
            .then(res => res.json())
            .then(status => setStatus(status))
            .catch(error => console.warn('--e--> ', error))
    }

    const setScheduling = (scheduling: boolean) => {
        setStatus(Object.assign({}, status, { scheduling }))
    }

    useEffect(() => fetchStatus(), [])

    if (status == null) {
        return <b>Loading ...</b>
    }

    return (
        <div>
            <StatusTable status={status} />
            <LoginButton authorized={ status.authorized } />
            <ToggleButton scheduling={ status.scheduling } setScheduling={setScheduling} />
        </div>
    )
}
