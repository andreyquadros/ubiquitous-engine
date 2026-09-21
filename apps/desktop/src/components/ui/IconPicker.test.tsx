import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '../../i18n';
import { ICON_CHOICES } from '../../lib/categories';
import { IconPicker } from './IconPicker';

describe('IconPicker', () => {
  it('renders every curated icon with a Portuguese name and marks the selected one', () => {
    const onChange = vi.fn();
    render(<IconPicker value="rocket" onChange={onChange} color="#F97316" />);
    expect(screen.getAllByRole('radio')).toHaveLength(ICON_CHOICES.length);
    expect(screen.getByRole('radio', { name: 'Foguete' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'Formatura' })).toHaveAttribute('aria-checked', 'false');
    fireEvent.click(screen.getByRole('radio', { name: 'Café' }));
    expect(onChange).toHaveBeenCalledWith('coffee');
  });

  it('moves with the arrow keys', () => {
    const onChange = vi.fn();
    render(<IconPicker value="graduation-cap" onChange={onChange} />);
    fireEvent.keyDown(screen.getByRole('radio', { name: 'Formatura' }), { key: 'ArrowRight' });
    expect(onChange).toHaveBeenCalledWith('rocket');
  });

  it('names the icons in English when the locale is en', () => {
    setLocale('en');
    render(<IconPicker value="briefcase" onChange={() => {}} />);
    expect(screen.getByRole('radiogroup', { name: 'Icon' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Briefcase' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'Graduation cap' })).toBeInTheDocument();
  });
});
