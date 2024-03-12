type LoginButtonProps = {
    loginUrl: string
    authorized: boolean
}

export const LoginButton = ({ loginUrl, authorized }: LoginButtonProps) => (
    <button disabled={authorized} onClick={() => { window.location.href = loginUrl }}>
        Connect with Strava
    </button>
)