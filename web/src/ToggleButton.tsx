const TOGGLE_URL = 'http://localhost:3000/toggle'

type ToggleButtonProps = {
    disabled: boolean,
    schedulerState: string,
    setSchedulerState (schedulerState: string)
}

export const ToggleButton = ({ disabled, schedulerState, setSchedulerState }: ToggleButtonProps) => {
    const toggle = () => fetch(TOGGLE_URL)
        .then(res => res.text())
        .then(result => setSchedulerState(JSON.parse(result)))
        .catch(error => console.warn('--e--> ', error))

    return (
        <button disabled={disabled} onClick={toggle}>
            { schedulerState === 'Inactive' ? 'Start scheduler' : 'Stop scheduler'}
        </button>
    )
}
