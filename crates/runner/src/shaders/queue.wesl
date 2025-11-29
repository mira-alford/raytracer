struct Queue {
    read_counter: atomic<u32>,
    write_counter: atomic<u32>,
    data: array<u32>,
};

fn queue_pop(queue: ptr<storage, Queue>, out: ptr<storage, u32>) -> bool {
    let i = atomicAdd(&queue.read_counter, 1);
    let j = atomicLoad(&queue.write_counter);

    // read head greater than write
    if i >= j {
        return false;
    }

    out = queue.data[i];
    return true;
}

fn queue_reset(queue: ptr<storage, Queue>) {
    atomicStore(&queue.read_counter, 0);
    atomicStore(&queue.write_counter, 0);
}

fn queue_push(queue: ptr<storage, Queue>, in: u32) -> bool {
    let i = atomicAdd(&queue.write_counter, 1);

    // write head greater than capacity
    if i >= arrayLength(queue.data) {
        return false;
    }

    queue.data[i] = in;
    return true;
}
