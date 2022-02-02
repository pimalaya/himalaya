local M = {}

local actions = require('telescope.actions')
local action_state = require('telescope.actions.state')
local finders = require('telescope.finders')
local pickers = require('telescope.pickers')
local sorters = require('telescope.sorters')
local previewers = require('telescope.previewers')

local function preview_command(entry, bufnr)
  vim.api.nvim_buf_call(bufnr, function()
    local page = 0 -- page 0 for preview
    local account = vim.fn['himalaya#account#curr']()
    local success, output = pcall(vim.fn['himalaya#msg#list_with'], account, entry.value, page, true)
    if not (success) then
      vim.cmd('redraw')
      vim.bo.modifiable = true
      local errors = vim.fn.split(output, '\n')
      errors[1] = "Errors: "..errors[1]
      vim.api.nvim_buf_set_lines(bufnr, 0, -1, true, errors)
    end
  end)
end

local function entry_maker(entry)
  return {
    value = entry,
    display = entry,
    ordinal = entry,
    preview_command = preview_command,
  }
end

M.mbox_picker = function(cb, mboxes)
  local finder_opts = {results = mboxes}
  local previewer = nil
  if vim.g.himalaya_telescope_preview_enabled then
    finder_opts.entry_maker = entry_maker
    previewer = previewers.display_content.new({})
  end
  pickers.new {
    results_title = 'Mailboxes',
    finder = finders.new_table(finder_opts),
    sorter = sorters.get_generic_fuzzy_sorter(),
    attach_mappings = function(prompt_bufnr)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        vim.fn[cb](selection.display)
      end)

      return true
    end,
    previewer = previewer,
  }:find()
end

return M
