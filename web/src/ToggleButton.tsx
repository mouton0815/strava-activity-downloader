const TOGGLE_URL = 'http://localhost:2525/toggle'

type ToggleButtonProps = {
    disabled: boolean,
    downloadState: string,
    setDownloadState (state: string)
}

export const ToggleButton = ({ disabled, downloadState, setDownloadState }: ToggleButtonProps) => {
    const toggle = () => fetch(TOGGLE_URL)
        .then(res => res.text())
        .then(result => setDownloadState(JSON.parse(result)))
        .catch(error => console.warn(error))

    return (
        <button disabled={disabled} onClick={toggle}>
            { downloadState === 'Inactive' ? 'Start downloading' : 'Stop downloading'}
        </button>
    )
}
