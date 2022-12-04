
import { ChangeEventHandler, useState, useEffect, FC } from "react"
import type { Todo } from '../types/todo'
import {
    Button, Card, Checkbox,
    Grid, Modal, Stack,
    Typography, TextField,
} from "@mui/material"
import { modalInnerStyle } from "../styles/modal";
import { Box } from "@mui/system";

type Props = {
    todo: Todo
    onUpdate: (todo: Todo) => void
    onDelete: (id: number) => void
}

const TodoItem: FC<Props> = ({ todo, onUpdate, onDelete }) => {
    const [editing, setEditing] = useState(false)
    const [editText, setEditText] = useState('')

    useEffect(() => {
        setEditText(todo.text)
    }, [todo])

    const handleCompletedCheckbox: ChangeEventHandler = (e) => {
        onUpdate({
            ...todo,
            completed: !todo.completed
        })
    }

    const onCloseEditModal = () => {
        onUpdate({
            ...todo,
            text: editText,
        })
        setEditing(false)
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
                <Stack direction="row" spacing={1}>
                    <Button onClick={() => setEditing(true)} color="info">
                        edit
                    </Button>
                    <Button onClick={handleDelete} color="error">
                        delete
                    </Button>
                </Stack>
            </Grid>

            <Modal open={editing} onClose={onCloseEditModal}>
                <Box sx={modalInnerStyle}>
                    <Stack spacing={2}>
                        <TextField
                            size="small"
                            label="todo text"
                            defaultValue={todo.text}
                            onChange={(e) => setEditText(e.target.value)}
                        ></TextField>
                    </Stack>
                </Box>
            </Modal>
        </Card>
    )
}

export default TodoItem
