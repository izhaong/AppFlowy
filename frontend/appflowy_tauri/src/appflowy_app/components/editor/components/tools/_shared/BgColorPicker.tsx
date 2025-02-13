import React from 'react';
import { useTranslation } from 'react-i18next';
import { ColorPicker, ColorPickerProps } from '$app/components/editor/components/tools/_shared/ColorPicker';

export function BgColorPicker(props: ColorPickerProps) {
  const { t } = useTranslation();

  return <ColorPicker {...props} label={t('editor.backgroundColor')} />;
}
