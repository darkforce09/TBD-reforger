/*
 * T-090.1.2 — freestanding BC7 decoder for wasm32.
 * Wraps the public-domain bcdec.h (iOrange/bcdec v0.97) with a tiny bump
 * allocator + a whole-image BC7 decode entry point, so Node can decode the
 * BC7 mip surfaces extracted from Eden `_supertexture.edds` cells with no
 * native deps (built by build-bcdec-wasm.sh -> bcdec.wasm).
 *
 * Exports (see build flags):
 *   memory            linear memory (read/write from JS)
 *   reset()           rewind the bump allocator
 *   alloc(n) -> ptr   16-byte-aligned scratch within linear memory
 *   decode_bc7_image(src, w, h, dst)
 *                     src = w*h bytes of BC7 blocks (16B / 4x4 block),
 *                     dst = w*h*4 bytes RGBA8. w,h must be multiples of 4.
 */
#define BCDEC_IMPLEMENTATION
#include "bcdec.h"

/* wasm-ld provides __heap_base at the end of static data. */
extern unsigned char __heap_base;
static unsigned char *g_bump = 0;

__attribute__((export_name("reset"))) void reset(void) {
    g_bump = &__heap_base;
}

__attribute__((export_name("alloc"))) void *alloc(int n) {
    if (!g_bump) g_bump = &__heap_base;
    unsigned char *p = g_bump;
    g_bump += (unsigned)(n + 15) & ~((unsigned)15);
    return p;
}

__attribute__((export_name("decode_bc7_image")))
void decode_bc7_image(const unsigned char *src, int w, int h, unsigned char *dst) {
    const int pitch = w * 4;            /* RGBA8 bytes per dst row */
    const int blocksX = w / 4;
    const int blocksY = h / 4;
    const unsigned char *s = src;
    for (int by = 0; by < blocksY; ++by) {
        for (int bx = 0; bx < blocksX; ++bx) {
            bcdec_bc7(s, dst + (by * 4) * pitch + (bx * 4) * 4, pitch);
            s += 16;                    /* BC7 block = 16 bytes */
        }
    }
}
