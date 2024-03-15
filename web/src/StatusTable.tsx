import { ReactElement } from 'react'
import { ServerStatus } from './ServerStatus'

type StatusTableProps = {
    status: ServerStatus
}

export const StatusTable = ({ status }: StatusTableProps): ReactElement => (
    <table>
        <tbody>
        <tr>
            <th colSpan={2}>Server status</th>
        </tr>
        <tr>
            <td>Connected with Strava:</td>
            <td>{connectionText(status.authorized)}</td>
        </tr>
        <tr>
            <td>Download scheduler status:</td>
            <td>{downloaderText(status.download_state)}</td>
        </tr>
        <tr>
            <td>Number of downloaded activities:</td>
            <td><b>{status.activity_stats.act_count}</b></td>
        </tr>
        <tr>
            <td>Number of downloaded tracks:</td>
            <td><b>{status.activity_stats.trk_count}</b></td>
        </tr>
        <tr>
            <td>Date of oldest downloaded activity:</td>
            <td><b>{extractDate(status.activity_stats.act_min_time)}</b></td>
        </tr>
        <tr>
            <td>Date of latest downloaded activity:</td>
            <td><b>{extractDate(status.activity_stats.act_max_time)}</b></td>
        </tr>
        <tr>
            <td>Date of latest downloaded track:</td>
            <td><b>{extractDate(status.activity_stats.trk_max_time)}</b></td>
        </tr>
        </tbody>
    </table>
)

function connectionText(connected: boolean): ReactElement {
    return connected
        ? <b style={{color: 'darkgreen'}}>Connected</b>
        : <b style={{color: 'darkred'}}>Disconnected</b>
}
const extractDate = (datetime: string | null): string => {
    return datetime ? datetime.substring(0, 10) : ''
}

function downloaderText(status: string): ReactElement {
    switch (status) {
        case 'Inactive': return (
            <b>Inactive</b>
        )
        case 'NoResults': return (
            <b>No further activities</b>
        )
        case 'LimitReached': return (
            <>
                <b style={{ color: 'darkred' }}>Strava API limit reached</b>
                <div>Please retry later</div>
            </>
        )
        case 'RequestError': return (
            <>
                <b style={{ color: 'darkred' }}>Strava API returned error</b>
                <div>Please inspect the server log</div>
            </>
        )
        case 'Activities': return (
            <b style={{color: 'darkgreen'}}>Activity download</b>
        )
        case 'Tracks': return (
            <b style={{color: 'darkgreen'}}>Track download</b>
        )
        default: throw new Error('Illegal state')
    }
}
