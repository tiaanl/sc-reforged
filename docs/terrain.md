gridData = [u32; terrain_width * terrain_height];

all values initialized to: 0x0001ff00

height lookup value = *puVar2 & 0xffffff00 | (uint)(byte)(-b - 1);

b = byte read from pcx
gridData[i] = gridData[i] & 0xffffff00 | (-b - 1)

something (8 bits)
normal lookup (16 bits)
height lookup (8 bits)


normal indices are set: this->m_gridData[index] & 0xfffc00ff | (uint)normal_lookup << 8;

get normal index: this->m_gridData[index] >> 8 & 0x3ff;
