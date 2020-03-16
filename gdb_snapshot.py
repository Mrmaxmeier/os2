# depends on GEF!
import gdb
import dataclasses
import base64
import json
import hashlib
from dataclasses import dataclass
from typing import Dict

# https://stackoverflow.com/a/51286749
class EnhancedJSONEncoder(json.JSONEncoder):
    def default(self, o):
        if dataclasses.is_dataclass(o):
            return dataclasses.asdict(o)
        return super().default(o)


def chunks(lst, n):
    """Yield successive n-sized chunks from lst."""
    for i in range(0, len(lst), n):
        yield lst[i : i + n]


@dataclass
class Regs:
    rax: int
    rbx: int
    rcx: int
    rdx: int
    rsp: int
    rbp: int
    rsi: int
    rdi: int
    rip: int
    r8: int
    r9: int
    r10: int
    r11: int
    r12: int
    r13: int
    r14: int
    r15: int
    rflags: int


X86_64_REGS = list(Regs.__annotations__)


@dataclass
class Mapping:
    perm_r: bool
    perm_w: bool
    perm_x: bool
    page_start: int
    page_end: int
    hint: str
    pages: [int]


@dataclass
class Snapshot:
    regs: Regs
    maps: [Mapping]
    chunks: Dict[int, str]


class SnapshotSave(gdb.Command):
    def __init__(self):
        super(SnapshotSave, self).__init__("snapshot-save", gdb.COMMAND_USER)

    def invoke(self, arg, from_tty):
        path = arg or "/tmp/snapshot.json"
        maps = []
        chunk_data = {}
        for procmap in get_process_maps():
            if not procmap.permission & Permission.READ:
                print("unable to dump", procmap.path, "at", hex(procmap.page_start))
                continue
            gdb.execute(
                f"dump memory /tmp/snapshot-region.bin {procmap.page_start} {procmap.page_end}"
            )
            with open("/tmp/snapshot-region.bin", "rb") as f:
                data = f.read()
            pages = []
            for chunk in chunks(data, 4096):
                key = int(hashlib.sha256(chunk).hexdigest()[:8], 16)
                chunk = base64.b64encode(chunk).decode("ascii")
                pages.append(key)
                chunk_data[key] = chunk
            maps.append(
                Mapping(
                    pages=pages,
                    page_start=procmap.page_start,
                    page_end=procmap.page_end,
                    perm_r=(procmap.permission & Permission.READ) != 0,
                    perm_w=(procmap.permission & Permission.WRITE) != 0,
                    perm_x=(procmap.permission & Permission.EXECUTE) != 0,
                    hint=procmap.path,
                )
            )
        registers = {
            k: int(gdb.parse_and_eval("$" + k.replace("rflags", "eflags")))
            for k in X86_64_REGS
        }
        snapshot = Snapshot(regs=Regs(**registers), maps=maps, chunks=chunk_data,)
        with open(path, "w") as f:
            print("saving to", path)
            json.dump(snapshot, f, indent=2, sort_keys=True, cls=EnhancedJSONEncoder)


snapshot_save = SnapshotSave()
