import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '../../i18n';
import { LanguageSelect } from './LanguageSelect';

describe('LanguageSelect', () => {
  it('lists both languages in their own words and reports the pick', () => {
    const onChange = vi.fn();
    render(<LanguageSelect value="pt-BR" onChange={onChange} />);
    const group = screen.getByRole('radiogroup', { name: 'Idioma' });
    expect(group).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Português (Brasil)' })).toHaveAttribute('aria-checked', 'true');
    const en = screen.getByRole('radio', { name: 'English' });
    expect(en).toHaveAttribute('aria-checked', 'false');
    fireEvent.click(en);
    expect(onChange).toHaveBeenCalledWith('en');
  });

  it('names the group in English when the locale is en', () => {
    setLocale('en');
    render(<LanguageSelect value="en" onChange={() => {}} compact />);
    expect(screen.getByRole('radiogroup', { name: 'Language' })).toBeInTheDocument();
  });
});
