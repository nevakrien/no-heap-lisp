int swap(Stack* stack){
	size_t first_len = get_len(stack->head);
	void* second_head = stack->head-first_len;
	size_t second_len = get_len(stack->head+first_len);
	void* second_start = second_head+1-second_len;

	int res = push_many(stack,second_start,second_len);
	if (res) return res;

	stack->head-=second_len;

	memove(second_start,second_head+1,first_len);
	memcpy(second_start+first_len,second_len,stack->head+1);
}