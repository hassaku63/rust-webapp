
import { ChangeEventHandler, FC } from "react"
import type { Todo } from '../types/todo'
import { Button, Card, Checkbox, Grid, Stack, Typography } from "@mui/material"

type Props = {
    todo: Todo
    onUpdate: (todo: Todo) => void
    onDelete: (id: number) => void
}

const TodoItem: FC<Props> = ({ todo, onUpdate, onDelete }) => {
    const handleCompletedCheckbox: ChangeEventHandler = (e) => {
        onUpdate({
            ...todo,
            completed: !todo.completed
        })
    }

    const handleDelete = () => onDelete(todo.id)

    return (
        <Card key={todo.id} sx={{ p: 1 }}>
            <Grid container spacing={2} alignItems="center">
                <Grid item xs={1}>
                    <Checkbox
                        checked={todo.completed}
                        onChange={handleCompletedCheckbox}
                    />
                    <Typography variant="body1">{todo.text}</Typography>
                </Grid>
            </Grid>

            <Grid item xs={9}>
                <Stack spacing={1}>
                    <Typography variant="caption" fontSize={16}>
                        {todo.text}
                    </Typography>
                </Stack>
            </Grid>

            <Grid item xs={1}>
                <Button onClick={handleDelete} color="error">
                    delete
                </Button>
            </Grid>
        </Card>
    )
}

export default TodoItem
