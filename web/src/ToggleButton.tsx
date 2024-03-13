type ToggleButtonProps = {
    toggleUrl: string
    disabled: boolean,
    downloadState: string,
    setDownloadState (state: string): void
}

export const ToggleButton = ({ toggleUrl, disabled, downloadState, setDownloadState }: ToggleButtonProps) => {
    const toggle = () => fetch(toggleUrl)
        .then(res => res.text())
        .then(result => setDownloadState(JSON.parse(result)))
        .catch(error => console.warn(error))

    return (
        <button disabled={disabled} onClick={toggle}>
            { isActive(downloadState) ? 'Stop downloading' : 'Start downloading'}
        </button>
    )
}

function isActive(status: string): boolean {
    switch (status) {
        case 'Inactive': return false
        case 'NoResults': return false
        case 'LimitReached': return false
        case 'Activities': return true
        case 'Tracks': return true
        default: throw new Error('Illegal state')
    }
}

