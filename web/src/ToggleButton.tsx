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
            { downloadState === 'Inactive' ? 'Start downloading' : 'Stop downloading'}
        </button>
    )
}
