const LOGIN_URL = 'http://localhost:3000/authorize'

type LoginButtonProps = {
    authorized: boolean
}

export const LoginButton = ({ authorized }: LoginButtonProps) => (
    <button disabled={authorized} onClick={() => { window.location = LOGIN_URL }}>
        Login to Strava
    </button>
)