import {ServerStatus} from './Status'

type StatusTableProps = {
    status: ServerStatus
}

const formatTime = (time: string | null): string => {
    return time ? time.replace('T', ' ').replace('Z', '') : ''
}

export const StatusTable = ({ status }: StatusTableProps) => (
    <table>
        <tbody>
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
        </tbody>
    </table>
)

