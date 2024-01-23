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
            <td>Connected with Strava:</td>
            <td><b>{ Boolean(status.authorized).toString() }</b></td>
        </tr>
        <tr>
            <td>Download scheduler status:</td>
            <td><b>{ downloaderText(status.download_state) }</b></td>
        </tr>
        <tr>
            <td>Number of downloaded activities:</td>
            <td><b>{ status.activity_stats.count }</b></td>
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

function downloaderText(status: string): String {
    switch (status) {
        case 'Inactive': return 'inactive'
        case 'Activities': return 'activity download'
        case 'Tracks': return 'track download'
    }
}
