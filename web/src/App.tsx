import {FunctionComponent, useEffect, useState} from 'react'
//import './App.css'
import {ServerStatus} from './Status'

type LoginButtonProps = {
    authorized: boolean
}

const LoginButton: FunctionComponent<LoginButtonProps> = ({authorized}) => {
    if (authorized) {
        return null
    }
    return (
        <button onClick={() => { window.location = 'http://localhost:3000/authorize' }}>
            Login at Strava
        </button>
    )
}

const App: FunctionComponent = () => {
    const statusUrl = "http://localhost:3000/status"
    const toggleUrl = "http://localhost:3000/toggle"
    const [status, setStatus] = useState<ServerStatus | null>(null)

    const fetchStatus = () => {
        fetch(statusUrl)
            .then(res => res.json())
            .then(status => setStatus(status))
            .catch(error => console.warn('--e--> ', error))
    }

    const toggle = () => {
        fetch(toggleUrl)
            .then(res => res.text())
            .then(result => setStatus(Object.assign({}, status, { scheduling: result == 'true' })))
            .catch(error => console.warn('--e--> ', error))
    }

    useEffect(() => fetchStatus(), [])

    if (status == null) {
        return <b>Loading ...</b>
    }
    const formatTime = (time: string | null): string => {
        return time ? time.replace('T', ' ').replace('Z', '') : ''
    }

    return (
        <div>
            <ul>
                <li>Authorized: <b>{ Boolean(status.authorized).toString() }</b></li>
                <li>Scheduling: <b>{ Boolean(status.scheduling).toString() }</b></li>
                <li>Activities: <b>{ status.activity_stats.count }</b></li>
                <li>Earliest: <b>{ formatTime(status.activity_stats.min_time) }</b></li>
                <li>Latest: <b>{ formatTime(status.activity_stats.max_time) }</b></li>
            </ul>
            <LoginButton authorized={ status.authorized } />
            <button onClick={toggle}>
                { status.scheduling ? 'Stop scheduler' : 'Start scheduler'}
            </button>
        </div>
    )
}

export default App
