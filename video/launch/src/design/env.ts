import {getInputProps} from 'remotion';

/**
 * Draft mode: `npm run render:draft` passes `--props='{"draft":true}'`.
 * Expensive effects (MotionBlur samples, heavy blurs) should check this.
 */
export const isDraft = (): boolean => {
	try {
		return Boolean((getInputProps() as {draft?: boolean}).draft);
	} catch {
		return false;
	}
};
