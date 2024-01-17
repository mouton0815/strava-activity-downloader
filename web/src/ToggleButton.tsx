const TOGGLE_URL = 'http://localhost:3000/toggle'

type ToggleButtonProps = {
    disabled: boolean,
    scheduling: boolean,
    setScheduling (scheduling: boolean)
}

export const ToggleButton = ({ disabled, scheduling, setScheduling }: ToggleButtonProps) => {
    const toggle = () => fetch(TOGGLE_URL)
        .then(res => res.text())
        .then(result => setScheduling(result === 'true'))
        .catch(error => console.warn('--e--> ', error))

    return (
        <button disabled={disabled} onClick={toggle}>
            { scheduling ? 'Stop scheduler' : 'Start scheduler'}
        </button>
    )
}
