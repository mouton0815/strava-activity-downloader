import {FunctionComponent, useEffect, useState} from 'react'
//import './App.css'
import {ServerStatus} from './Status'

type LoginButtonProps = {
    authorized: boolean
}

const LoginButton: FunctionComponent<LoginButtonProps> = ({authorized}) => (
    <button disabled={authorized} onClick={() => { window.location = 'http://localhost:3000/authorize' }}>
        Login at Strava
    </button>
)

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
            <table>
                <tr>
                    <th colSpan={2}>Server status</th>
                </tr>
                <tr>
                    <td>Authenticated with Strava:</td>
                    <td><b>{ Boolean(status.authorized).toString() }</b></td>
                </tr>
                <tr>
                    <td>Download scheduler running:</td>
                    <td><b>{ Boolean(status.scheduling).toString() }</b></td>
                </tr>
                <tr>
                    <td>Number of downloaded activities:</td>
                    <td><b>{ status.activity_stats.count }</b></td>
                </tr>
                <tr>
                    <td>Date and time of earliest activity:</td>
                    <td><b>{ formatTime(status.activity_stats.min_time) }</b></td>
                </tr>
                <tr>
                    <td>Date and time of latest activity:</td>
                    <td><b>{ formatTime(status.activity_stats.max_time) }</b></td>
                </tr>
            </table>
            <LoginButton authorized={ status.authorized } />
            <button onClick={toggle}>
                { status.scheduling ? 'Stop scheduler' : 'Start scheduler'}
            </button>
        </div>
    )
}

export default App
