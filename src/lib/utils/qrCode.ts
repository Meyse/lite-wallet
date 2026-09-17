export const QR_QUIET_ZONE_MODULES = 4;

export interface QrCodeMatrix {
  size: number;
  modules: Uint8Array;
}

export interface RenderedQrCode extends QrCodeMatrix {
  path: string;
  quietZone: number;
  viewBoxSize: number;
}

export function qrMatrixToPath(matrix: QrCodeMatrix, quietZone = QR_QUIET_ZONE_MODULES): string {
  const commands: string[] = [];

  for (let row = 0; row < matrix.size; row += 1) {
    for (let column = 0; column < matrix.size; column += 1) {
      if (!matrix.modules[row * matrix.size + column]) continue;
      commands.push(`M${column + quietZone} ${row + quietZone}h1v1h-1z`);
    }
  }

  return commands.join('');
}

export async function encodeQrCode(value: string): Promise<RenderedQrCode> {
  if (!value) throw new Error('QR payload is empty.');

  const { create } = await import('qrcode');
  const encoded = create(value, { errorCorrectionLevel: 'M' });
  const matrix: QrCodeMatrix = {
    size: encoded.modules.size,
    modules: Uint8Array.from(encoded.modules.data),
  };

  return {
    ...matrix,
    quietZone: QR_QUIET_ZONE_MODULES,
    viewBoxSize: matrix.size + QR_QUIET_ZONE_MODULES * 2,
    path: qrMatrixToPath(matrix),
  };
}
