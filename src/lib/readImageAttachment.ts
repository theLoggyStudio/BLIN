/** Pièce jointe image pour la barre de commande (analyse vision). */
export interface CommandBarImageAttachment {
  dataUrl: string;
  mimeType: string;
  fileName: string;
  previewUrl: string;
}

const MAX_BYTES = 4 * 1024 * 1024;

export async function readImageAttachment(file: File): Promise<CommandBarImageAttachment> {
  if (!file.type.startsWith("image/")) {
    throw new Error("Choisissez un fichier image (PNG, JPEG, WebP, GIF).");
  }
  if (file.size > MAX_BYTES) {
    throw new Error("Image trop volumineuse (max 4 Mo).");
  }
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(new Error("Impossible de lire l'image."));
    reader.readAsDataURL(file);
  });
  return {
    dataUrl,
    mimeType: file.type || "image/png",
    fileName: file.name,
    previewUrl: dataUrl,
  };
}
