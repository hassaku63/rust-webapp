import { useEffect, useState, FC } from 'react'
import 'modern-css-reset'
import { ThemeProvider, createTheme } from '@mui/material/styles'
import { Box, Grid, Stack, Typography } from "@mui/material";
import { Label, NewLabelPayload, NewTodoPayload, UpdateTodoPayload, Todo } from "./types/todo";
import TodoList from "./components/TodoList";
import TodoForm from "./components/TodoForm";
import SideNav from "./components/SideNav";
import {
  addTodoItem,
  getTodoItems,
  updateTodoItem,
  deleteTodoItem
} from "./lib/api/todo";
import { addLabelItem, deleteLabelItem, getLabelItems } from "./lib/api/label";

const TodoApp: FC = () => {
  const [todos, setTodos] = useState<Todo[]>([])
  const [labels, setLabels] = useState<Label[]>([])
  const [filterLabelId, setFilterLabelId] = useState<number | null>(null)

  const onSubmit = async (payload: NewTodoPayload) => {
    if (!payload.text) return
    const newTodo = await addTodoItem(payload)
    const todos = await getTodoItems()
    setTodos(todos)
  }

  const onUpdate = async (updateTodo: UpdateTodoPayload) => {
    await updateTodoItem(updateTodo.id, {
      id: updateTodo.id,
      text: updateTodo.text,
      completed: updateTodo.completed,
      labels: updateTodo.labels,
    })

    const todos = await getTodoItems()
    setTodos(todos)
  }

  const onDelete = async (id: number) => {
    await deleteTodoItem(id)
    const todos = await getTodoItems()
    setTodos(todos)
  }

  // マウント後に Todo アイテムのフェッチを実行している
  // useEffect の第2引数はフックとなるイベントをリスト指定するらしい
  // 空の配列の場合だと、マウント時とアンマウント時(DOM の廃棄)のみ、という意味になる
  useEffect(() => {
    (async () => {
      const todos = await getTodoItems()
      setTodos(todos)

      const labelResponse = await getLabelItems()
      setLabels(labelResponse)
    })()
  }, [])

  const onSelectLabel = (label: Label | null) => {
    setFilterLabelId(label?.id ?? null)
  }

  const onSubmitNewLabel = async (newLabel: NewLabelPayload) => {
    if (!labels.some(label => label.name === newLabel.name)) {
      const res = await addLabelItem(newLabel)
      setLabels([...labels, res])
    }
  }

  const onDeleteLabel = async (id: number) => {
    await deleteLabelItem(id)
    setLabels((prev) => prev.filter((label) => label.id !== id))
  }

  const dispTodo = filterLabelId ?
    todos.filter((todo) => 
      todo.labels.some((label) => label.id === filterLabelId)
    )
    : todos

  return (
    <>
      <Box
        sx={{
          backgroundColor: 'white',
          borderBottom: '1px solid gray',
          display: 'flex',
          alignItems: 'center',
          position: 'fixed',
          top: 0,
          p: 2,
          width: '100%',
          height: 80,
          zIndex: 3,
        }}
      >
        <Typography variant='h1'>Todo App</Typography>
      </Box>

      <Box
        sx={{
          backgroundColor: 'white',
          boarderRight: '1px solid gray',
          position: 'flex',
          height: 'calc(100% - 80px)',
          width: 200,
          zIndex: 2,
          left: 0,
        }}
      >
        <SideNav
          labels={labels}
          onSelectLabel={onSelectLabel}
          filterLabelId={filterLabelId}
          onSubmitNewLabel={onSubmitNewLabel}
          onDeleteLabel={onDeleteLabel}
        />
      </Box>

      <Box
        sx={{
          display: 'flex',
          justifyContent: 'center',
          p: 5,
          mt: 10,
        }}
      >
        <Box maxWidth={700} width="100%">
          <Stack spacing={5}>
            <TodoForm onSubmit={onSubmit} labels={labels} />
            <TodoList
              todos={dispTodo}
              labels={labels}
              onUpdate={onUpdate}
              onDelete={onDelete}
            />
          </Stack>
        </Box>
      </Box>
    </>
  )
}

const theme = createTheme({
  typography: {
    h1: {
      fontSize: 30,
    },
    h2: {
      fontSize: 20,
    },
  },
})

const App: FC = () => {
  return <ThemeProvider theme={theme}>
    <TodoApp></TodoApp>
  </ThemeProvider>
}

export default App
