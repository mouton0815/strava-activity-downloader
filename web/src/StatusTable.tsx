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
            <td>{ connectionText(status.authorized) }</td>
        </tr>
        <tr>
            <td>Download scheduler status:</td>
            <td>{ downloaderText(status.download_state) }</td>
        </tr>
        <tr>
            <td>Number of downloaded activities:</td>
            <td><b>{ status.activity_stats.act_count }</b></td>
        </tr>
        <tr>
            <td>Number of downloaded tracks:</td>
            <td><b>{ status.activity_stats.trk_count }</b></td>
        </tr>
        <tr>
            <td>Date of earliest downloaded activity:</td>
            <td><b>{ extractDate(status.activity_stats.min_time) }</b></td>
        </tr>
        <tr>
            <td>Date of latest downloaded activity:</td>
            <td><b>{ extractDate(status.activity_stats.max_time) }</b></td>
        </tr>
        </tbody>
    </table>
)

function connectionText(connected: boolean): ReactElement {
    return connected ? <b>Connected</b> : <b style={{color: 'darkred'}}>Disconnected</b>
}
const extractDate = (datetime: string | null): string => {
    return datetime ? datetime.substring(0, 10) : ''
}

function downloaderText(status: string): ReactElement {
    switch (status) {
        case 'Inactive': return <b>Inactive</b>
        case 'NoResults': return <b>No further activities</b>
        case 'LimitReached': return (
            <>
                <b style={{ color: 'darkred' }}>API limit reached</b>
                <div>Please restart later</div>
            </>
        )
        case 'Activities': return <b>Activity download</b>
        case 'Tracks': return <b>Track download</b>
        default: throw new Error('Illegal state')
    }
}
