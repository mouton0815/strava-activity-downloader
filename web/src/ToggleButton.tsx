const TOGGLE_URL = 'http://localhost:3000/toggle'

type ToggleButtonProps = {
    scheduling: boolean,
    setScheduling (scheduling: boolean)
}

export const ToggleButton = ({ scheduling, setScheduling }: ToggleButtonProps) => {
    const toggle = () => fetch(TOGGLE_URL)
        .then(res => res.text())
        .then(result => setScheduling(result === 'true'))
        .catch(error => console.warn('--e--> ', error))

    return (
        <button onClick={toggle}>
            { scheduling ? 'Stop scheduler' : 'Start scheduler'}
        </button>
    )
}
