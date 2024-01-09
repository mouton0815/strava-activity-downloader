import {ServerStatus} from './Status'

type StatusTableProps = {
    status: ServerStatus
}

const extractDate = (datetime: string | null): string => {
    return datetime ? datetime.substring(0, 10) : ''
}

export const StatusTable = ({ status }: StatusTableProps) => (
    <table>
        <tbody>
        <tr>
            <th colSpan={2}>Server status</th>
        </tr>
        <tr>
            <td>Logged in to Strava:</td>
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
            <td>Date earliest downloaded activity:</td>
            <td><b>{ extractDate(status.activity_stats.min_time) }</b></td>
        </tr>
        <tr>
            <td>Date of latest downloaded activity:</td>
            <td><b>{ extractDate(status.activity_stats.max_time) }</b></td>
        </tr>
        </tbody>
    </table>
)

